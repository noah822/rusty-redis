pub mod server;

pub mod parser;
pub mod command;
pub mod persistence;


#[cfg(test)]
mod tests{
    use crate::parser::decrypt::parse_resp;
    use crate::parser::{SimpleRESPObject, AggrRESPObject, RESPObject, AtomicItem};
    use std::str; 
    
    #[test]
    fn parse_simple_str(){
        let ping = b"+PING\r\n";
        let parsed_res = parse_resp(ping).unwrap();
        match parsed_res {
            RESPObject::Simple(SimpleRESPObject::Str(content)) => {
                assert_eq!(content, "PING");
            },
            _ => {}
        }
    }
    
    #[test]
    fn parse_bulk_str(){
        let ping = b"$4\r\nPING\r\n";
        let parsed_res = parse_resp(ping).unwrap();
        match parsed_res {
            RESPObject::Aggregate(AggrRESPObject::BulkStr(content)) => {
                assert_eq!(content, b"PING");
            },
            _ => {}
        }
    }

    
    #[test]
    fn parse_array(){
        let array_str = b"*2\r\n$5\r\nhello\r\n$5\r\nworld\r\n";
        let parsed_res = parse_resp(array_str).unwrap();
        let unpacked = match parsed_res {
            RESPObject::Aggregate(AggrRESPObject::Array(ref v)) => {
                v.iter().take_while(|item|{
                    match item {
                        AtomicItem::SimpleItem(SimpleRESPObject::Str(_)) => true,
                        AtomicItem::AggrItem(AggrRESPObject::BulkStr(_)) => true,
                        _ => false
                    }
                }).map(|item| {item.as_bytes().unwrap()}).collect()
            },
            _ => {vec!()}
        };
        assert_eq!(unpacked, vec!(b"hello", b"world"));
    }


    
}
