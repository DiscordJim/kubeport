use httparse::Response;
use hyper::StatusCode;


pub fn basic_response(code: StatusCode) -> String {
    format!("HTTP/1.1 {}", code.as_str())
}

pub fn response_to_bytes(res: Response) -> String {
    let rstring = String::new();
    let status_line = format!("HTTP/1.1");

    rstring

    
}