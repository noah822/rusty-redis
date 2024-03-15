/* internal classification of RESPObject
 * Simple
 * Aggregate
 *      compound 
 *      single
 * */


pub enum SimpleRESPObject<'a>{
    Integer(i32),
    Str(&'a str),
}


pub enum AtomicItem<'a>{
    SimpleItem(SimpleRESPObject<'a>),
    AggrItem(AggrRESPObject<'a>)
}
pub enum AggrRESPObject<'a>{
   BulkStr(&'a [u8]),
   Array(Vec<AtomicItem<'a>>)
}


pub enum RESPObject<'a>{
    Simple(SimpleRESPObject<'a>),
    Aggregate(AggrRESPObject<'a>)
}


impl<'a> SimpleRESPObject<'a>{
    pub fn as_bytes(self) -> Option<&'a [u8]>{
       match self{
           Self::Str(s) => Some(s.as_bytes()),
           _ => None
       } 
    }
}

impl<'a> AggrRESPObject<'a>{
    pub fn as_bytes(self) -> Option<&'a [u8]>{
        match self{
            Self::BulkStr(s) => Some(s),
            _ => None
        }
    }
}

impl<'a> AtomicItem<'a>{
    pub fn as_bytes(self) -> Option<&'a [u8]>{
        match self{
            Self::SimpleItem(object) => object.as_bytes(),
            Self::AggrItem(object) => object.as_bytes()
        }
    }
}




pub mod decrypt{
    use super::{RESPObject, AggrRESPObject, SimpleRESPObject, AtomicItem};
    use std::str;

    fn extract_simple_object(content: &[u8]) -> Result<SimpleRESPObject, &'static str>{
       match content[0] {
           b'+' => {
               let str_content = str::from_utf8(&content[1..]).unwrap()
                                    .strip_suffix("\r\n").unwrap();
               Ok(SimpleRESPObject::Str(str_content))
           },
           _ => {todo!()}
       }
    }
    
    
    fn extract_single_aggregate_object(content: &[u8]) -> Result<AggrRESPObject, &'static str>{
        // including types of bulkstr, bulkerror 
        match content[0] {
           b'$' => {
               // bulk str: $<length>\r\n<data>\r\n
               let mut sstream = str::from_utf8(&content[1..]).unwrap().split("\r\n");
               let bstr_len = sstream.next().unwrap().parse::<i32>().unwrap();
               let bstr_content: &[u8] = sstream.next().unwrap().as_bytes(); 
               
               assert_eq!(bstr_len as usize, bstr_content.len());
               Ok(AggrRESPObject::BulkStr(bstr_content))
           },
           _ => {todo!()}
        }
    }
    
    
    fn extract_nested_object(content: &[u8]) -> Result<AggrRESPObject, &'static str>{
        match content[0] {
            b'*' => {
                // Array: *<num-items>\r\n<element-1>...<element-n>
                let sstream = str::from_utf8(&content[1..]).unwrap();
                let (num_item, sstream) = sstream.split_once("\r\n").unwrap();
                // sstream: <type><data>\r\n || <type><data>\r\n<data>\r\n
                
                
                let mut objects_arr = vec!();
                let mut sstream_iter = sstream.chars().enumerate();
                while let Some((start, category)) = sstream_iter.next(){
                    let (expect_num_terminator, generator): (i32, Box<dyn Fn(&[u8]) -> AtomicItem>) = match category {
                        '$' => (2, Box::new(|chunk|{
                            AtomicItem::AggrItem(extract_single_aggregate_object(chunk).unwrap())
                        })),
                         _  => (1, Box::new(|chunk|{
                             AtomicItem::SimpleItem(extract_simple_object(chunk).unwrap())
                         }))
                    };
                    
                    let mut num_terminator = 0i32;
                    while let Some((_, c)) = sstream_iter.next(){
                        if c == '\r' {
                            if let Some((next_index, next_c)) = sstream_iter.next(){
                               if next_c == '\n'{
                                   num_terminator += 1;
                               } 
                               if num_terminator == expect_num_terminator{
                                   objects_arr.push(generator(&sstream[start..next_index+1].as_bytes()));
                                   break;
                               }
                            }
                        }
                    }
                }
                assert_eq!(str::parse::<usize>(num_item).unwrap(), objects_arr.len());
                Ok(AggrRESPObject::Array(objects_arr))
                
            },
            _ => {todo!()}
        }
    }
    
    
    pub fn parse_resp(content: &[u8]) -> Result<RESPObject, &'static str>{
        
        let serialized_res = match content[0]{
           b':' | b'+' => {
               let simple_object = extract_simple_object(content)?;
               Ok(RESPObject::Simple(simple_object))
           },
           b'$' => {
               let single_aggr_object = extract_single_aggregate_object(content)?;
               Ok(RESPObject::Aggregate(single_aggr_object))
           }, 
           b'*' => {
               let aggr_object = extract_nested_object(content)?;
               Ok(RESPObject::Aggregate(aggr_object))
           },
            _ => {Err("unrecognized category indicator")}
        };
        serialized_res
    }
}



pub mod encrypt{
    use std::str;
    pub fn as_bulk_str(msg: &[u8]) -> Box<[u8]>{
       let size = msg.len();
       let s = format!("${size}").to_string() + "\r\n" + str::from_utf8(msg).unwrap() + "\r\n";
       s.into_bytes().into_boxed_slice()
    }

    pub fn as_simple_str(msg: &[u8]) -> Box<[u8]>{
        let s = '+'.to_string() + str::from_utf8(msg).unwrap() + "\r\n"; 
        s.into_bytes().into_boxed_slice()
    }
}

