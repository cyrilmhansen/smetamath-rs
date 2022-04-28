//! Utilities for source-offset/line-number mapping.

use std::cmp::Ordering;

use util::HashMap;

const PAGE: usize = 256;

/// An object for efficient repeated byte offset to line conversions.
///
/// The first time a query is made for a given buffer, an index is constructed
/// storing the line number at 256 byte intervals in the file.  Subsequent
/// queries can reuse the index.
///
/// This is expected to be a very short-lived object.  If the line cache
/// outlives any of the buffers it has been queried against, and future buffers
/// receive the same address range, the line cache will return incorrect results
/// (but will not crash).
#[derive(Default)]
pub struct LineCache {
    map: HashMap<(usize, usize), Vec<u32>>,
}

fn make_index(mut buf: &[u8]) -> Vec<u32> {
    assert!(buf.len() < u32::max_value() as usize - 1);
    let mut out = Vec::with_capacity(buf.len() / PAGE + 1);
    out.push(0);
    let mut count = 0u32;

    // record the running total of newlines every PAGE bytes
    while buf.len() >= PAGE {
        let mut page = &buf[0..PAGE];
        buf = &buf[PAGE..];
        // use an i8 accumulator to maximize the effectiveness of vectorization.
        // do blocks of 128 because we don't want to overflow the i8.  count
        // down because all vector hardware supported by Rust generates fewer
        // instructions that way (the natural compare instructions produce 0 and
        // -1, not 0 and 1).
        while page.len() >= 128 {
            let mut inner = 0i8;
            for &ch in &page[0..128] {
                inner += -((ch == b'\n') as i8);
            }
            page = &page[128..];
            count += (inner as u8).wrapping_neg() as u32;
        }
        out.push(count);
    }

    out
}

// find the lowest offset for which from_offset would give the target.
// Panics if line number out of range.
fn line_to_offset(buf: &[u8], index: &[u32], line: u32) -> usize {
    let page = index.binary_search_by(|&ll| if ll < line {
            Ordering::Less
        } else {
            Ordering::Greater
        })
        .err()
        .expect("cannot match");
    // page*PAGE is the first page-aligned which is >= to the goal line, OR it
    // points at an incomplete end page
    if page == 0 {
        // page 0 always has lineno 0, so inserting before it is only possible
        // if line is 0 or negative
        assert!(line == 0, "line out of range");
        return 0;
    }
    // (page-1)*PAGE, then, is *not* >= to the goal line, but it's close to
    // either the goal line or EOF
    let mut at_lineno = index[page - 1];
    let mut at_pos = (page - 1) * PAGE;
    while at_lineno < line {
        assert!(at_pos < buf.len(), "line out of range");
        if buf[at_pos] == b'\n' {
            at_lineno += 1;
        }
        at_pos += 1;
    }

    at_pos
}

impl LineCache {
    fn get_index(&mut self, buf: &[u8]) -> &Vec<u32> {
        self.map.entry((buf.as_ptr() as usize, buf.len())).or_insert_with(|| make_index(buf))
    }

    /// Map a line to a buffer index.  Panics if out of range.
    pub fn to_offset(&mut self, buf: &[u8], line: u32) -> usize {
        line_to_offset(buf, self.get_index(buf), line - 1)
    }

    /// Map a buffer index to a (line, column) pair.  Panics if the buffer is
    /// larger than 4GiB or if offset is out of range.
    pub fn from_offset(&mut self, buf: &[u8], offset: usize) -> (u32, u32) {
        let index = self.get_index(buf);
        // find a start point
        let mut lineno = index[offset / PAGE];
        // fine-tune
        for &ch in &buf[offset / PAGE * PAGE..offset] {
            if ch == b'\n' {
                lineno += 1;
            }
        }
        // now for the column
        let colno = offset - line_to_offset(buf, index, lineno);
        (lineno + 1, colno as u32 + 1)
    }


    /// Find the offset just after the end of the line (usually the
    /// location of a '\n', unless we are at the end of the file).
    pub fn line_end(buf: &[u8], offset: usize) -> usize {
        for (pos, car) in buf.iter().enumerate().skip(offset) {
            if *car == b'\n' {
                return pos;
            }
        }
        buf.len()
    }
}

#[cfg(test)]
mod tests {
    use line_cache::LineCache;

    use std::convert::TryInto;

    use rand::Rng;
    use rand::thread_rng;
    use rand::distributions::Alphanumeric;
    

    #[test]
    fn test_from_offset() {
        let mut lc = LineCache::default();
        let text = "azerty\r\nazerty4\r\nazerty3\r\n".as_bytes();
        
        let (row, col) = lc.from_offset(&text, 10);
        println!("{}:{}",row,col);
        assert!(row==2 && col == 3);

        let (row, col) = lc.from_offset(&text, 0);
        assert!(row==1 && col == 1);
        println!("{}:{}",row,col);

        let (row, col) = lc.from_offset(&text, 1);
        assert!(row==1 && col == 2);
        println!("{}:{}",row,col);

        let (row, col) = lc.from_offset(&text, 19);
        println!("{}:{}",row,col);
        assert!(row==3 && col == 3);

        //assert!(false);

    }

    #[test]
    fn test_line_end() {
        let mut lc = LineCache::default();
        let text = "azerty\r\nazerty4\r\nazerty3\r\n".as_bytes();

        let line_end = LineCache::line_end(&text, 10);
        assert!(line_end==16);
        println!("{}",line_end);

        let (row_a, col_a) = lc.from_offset(&text, line_end);
        println!("{}:{}", row_a, col_a);

        let (row_b, col_b) = lc.from_offset(&text, line_end+1);
        println!("{}:{}",row_b, col_b);
        assert!(row_b == row_a + 1);
        assert!(col_b == 1);

        //assert!(false);
    }

    #[test]
    fn test_large() {

        // Test using 40meg of 30 random ascii character lines
        let test_size = 40*1024*1024;
        let row_size = 30;
        let row_end = 31;
        let nb_rows = test_size / (row_size +1);

        let mut  row_buf : String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(row_size)
        .map(char::from)
        .collect();

        row_buf += "\n";

        let mut buf_text : String = String::new();

        for _ in 1..nb_rows {
            buf_text += &row_buf;
        }

        let text = buf_text.as_bytes();

        let mut lc = LineCache::default();

        let line_end = LineCache::line_end(&text, 20);
        println!("{}",line_end);
        assert!(line_end==30);

        let (row_a, col_a) = lc.from_offset(&text, line_end);
        println!("{}:{}",row_a,col_a);
        assert!(row_a == 1);
        assert!(col_a == row_end);

        let (row_b, col_b) = lc.from_offset(&text, line_end+1);
        println!("{}:{}",row_b,col_b);
        assert!(row_b == row_a+1);
        assert!(col_b == 1);

        for _ in 0..20 {
            // random offset
            let large_offset : usize = thread_rng().gen_range(0..test_size);

            let (row_c, col_c) = lc.from_offset(&text, large_offset);
            println!("{}:{}",row_c,col_c);

            let expected_row  =  1 + (large_offset / 31);
            let expected_col  =  1 + (large_offset % 31);
            println!("{}:{}", expected_row, expected_col);

            assert!(row_c == expected_row.try_into().unwrap());
            assert!(col_c == expected_col.try_into().unwrap());

            let row_end = LineCache::line_end(&text, large_offset);
            let expected_row_end  =  expected_row * 31 - 1;

            println!("{}:{}",expected_row_end, row_end);
            assert!(row_end == expected_row_end.try_into().unwrap());

        }

        //assert!(false);
    }


   

  

}