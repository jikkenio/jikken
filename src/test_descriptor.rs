use std::fs::File;
use std::io::{self, BufRead};
use hyper::Uri;
use super::config::Config;

#[derive(Debug)]
pub enum HttpVerb {
    GET,
    POST,
    PUT,
    PATCH,
    UNDEFINED
}

pub struct RequestDescriptor {
    pub verb: Option<HttpVerb>,
    pub url: Option<Uri>,
    pub params: Vec<(String, String)>,
    pub headers: Vec<(String, String)>,
    pub body: Option<Vec<u8>>,
}

impl RequestDescriptor {
    pub fn new() -> RequestDescriptor {
        RequestDescriptor { verb: None, url: None, params: Vec::new(), headers: Vec::new(), body: None }
    }
}

pub struct ResponseDescriptor {
    pub status_code: Option<u16>,
    pub headers: Vec<(String, String)>,
    pub body: Option<Vec<u8>>,
}

impl ResponseDescriptor {
    pub fn new() -> ResponseDescriptor {
        ResponseDescriptor { status_code: None, headers: Vec::new(), body: None }
    }
}

pub struct TestDescriptor {
    pub name: Option<String>,
    pub request: RequestDescriptor,
    pub request_comparison: Option<RequestDescriptor>,
    pub response: Option<ResponseDescriptor>,
    pub file: String,
    pub is_comparison: bool,
}

impl TestDescriptor {
    pub fn new(file: String) -> TestDescriptor {
        TestDescriptor { name: None, request: RequestDescriptor::new(), request_comparison: None, response: None, file: file, is_comparison: false }
    }

    pub fn load<'a>(&mut self, config: Option<Config>) {
        let file = File::open(&self.file);
        match file {
            Err(e) => println!("error loading file: {}", e),
            Ok(f) => {
                let lines = io::BufReader::new(f).lines();
                let mut multiline_body = false;
                for line in lines {
                    if let Ok(row) = line {
                        let mut r: String = if !multiline_body { row.trim_start().into() } else { row.into() };

                        match config {
                            Some(ref c) => {
                                if let Some(globals) = c.globals.as_ref() {
                                    for (key, value) in globals {
                                        let key_pattern = format!("#{}#", key);
                                        r = r.replace(&key_pattern, &value);
                                    }
                                }
                            }
                            _ => {}
                        }

                        match r.chars().next() {
                            Some('N') => {
                                multiline_body = false;
                                self.name = Some(r[2..].into());
                            },
                            Some('M') => {
                                multiline_body = false;
                                match r.chars().skip(1).next() {
                                    Some('C') => {
                                        if self.request_comparison.is_none() {
                                            self.request_comparison = Some(RequestDescriptor::new());
                                        }

                                        let request = self.request_comparison.as_mut().unwrap();

                                        let result = TestDescriptor::parse_url(&r, 4);
                                        match result {
                                            Some((verb, url)) => {
                                                let maybe_url = url.parse::<Uri>();
                                                match maybe_url {
                                                    Ok(u) => request.url = Some(u),
                                                    Err(_) => {
                                                        println!("Error: Invalid url in test definition ({} -> {}).", self.file, &r);
                                                        std::process::exit(exitcode::DATAERR);
                                                    }
                                                }
                                                request.verb = Some(verb);
                                                self.is_comparison = true;
                                                // println!("found secondary http verb and url ({:?}, {})", self.verb_secondary, url)
                                            },
                                            None => println!("unable to parse secondary http verb and url")  
                                        }
                                    },
                                    Some(' ') => {
                                        let result = TestDescriptor::parse_url(&r, 3);
                                        match result {
                                            Some((verb, url)) => {
                                                let maybe_url = url.parse::<Uri>();
                                                match maybe_url {
                                                    Ok(u) => self.request.url = Some(u),
                                                    Err(_) => {
                                                        println!("Error: Invalid url in test definition ({} -> {}).", self.file, &r);
                                                        std::process::exit(exitcode::DATAERR);
                                                    }
                                                }
                                                
                                                self.request.verb = Some(verb);
                                                // println!("found http verb and url ({:?}, {})", self.verb, url)
                                            },
                                            None => println!("unable to parse http verb and url")  
                                        }
                                    },
                                    Some(_) => {},
                                    None => {}
                                }
                            },
                            Some('P') => {
                                multiline_body = false;
                                let result = TestDescriptor::parse_key_value(&r);
                                match result {
                                    Some((key, value)) => {
                                        self.request.params.push((key.to_owned(), value.to_owned()));
                                        // println!("found url parameter ({} -> {})", key, value)
                                    },
                                    None => println!("unable to parse url parameter")
                                }
                            },
                            Some('H') => {
                                multiline_body = false;
                                match r.chars().skip(1).next() {
                                    Some('C') => {
                                        let result = TestDescriptor::parse_key_value(&r);
                                        match result {
                                            Some((key, value)) => {
                                                self.request.params.push((key.to_owned(), value.to_owned()));
                                                // println!("found http header ({} -> {})", key, value)
                                            },
                                            None => println!("unable to parse http header")
                                        }
                                    },
                                    Some(' ') => {
                                        let result = TestDescriptor::parse_key_value(&r);
                                        match result {
                                            Some((key, value)) => {
                                                self.request_comparison.as_mut().unwrap().headers.push((key.to_owned(), value.to_owned()));
                                                // println!("found http header ({} -> {})", key, value)
                                            },
                                            None => println!("unable to parse http header")
                                        }
                                    },
                                    Some(_) => {},
                                    None => {}
                                }
                            },
                            Some('#') => multiline_body = false, // println!("found comment"),
                            Some('R') => {
                                if self.response.is_none() {
                                    self.response = Some(ResponseDescriptor::new());
                                }

                                multiline_body = false;
                                let response = self.response.as_mut().unwrap();

                                match r.chars().skip(1).next() {
                                    Some('S') => {
                                        let result = TestDescriptor::parse_status_code(&r);
                                        match result {
                                            Some(r) => {
                                                response.status_code = Some(r);
                                                // println!("found response status code ({})", r)
                                            },
                                            None => println!("unable to parse response status code")
                                        }
                                    },
                                    Some('B') => {
                                        multiline_body = true;
                                        response.body = Some(Vec::new());
                                        response.body.as_mut().unwrap().extend_from_slice(r[3..].as_bytes());
                                    }
                                    Some(_) =>(), // println!("unknown response type"),
                                    None => () // println!("skip empty line")
                                }
                            },
                            Some(_) => {
                                if multiline_body {
                                    let response = self.response.as_mut().unwrap();
                                    response.body.as_mut().unwrap().extend_from_slice(r.as_bytes());
                                }
                            }, // println!("unknown line type"),
                            None => () // println!("skip empty line")
                        }
                    }
                }
            },
        }
    }

    fn parse_url(line: &str, offset: usize) -> Option<(HttpVerb, &str)> {
        let verb = line.split(' ').skip(1).next();
        let http_verb = match verb {
            Some("GET") => HttpVerb::GET,
            Some("POST") => HttpVerb::POST,
            Some("PUT") => HttpVerb::PUT,
            Some("PATCH") => HttpVerb::PATCH,
            Some(_) => return None,
            None => return None
        };

        let length = match verb {
            // this is a hardcoded offset for the prefix. this should be rewritten
            Some(x) => x.len() + offset, 
            None => 0
        };

        Some((http_verb, &line[length..]))
    }

    fn parse_key_value(line: &str) -> Option<(&str, &str)> {
        let key = line.split(' ').skip(1).next();

        let length = match key {
            // this is a hardcoded offset for the prefix. this should be rewritten
            Some(k) => k.len() + 3,
            None => return None
        };

        let value = &line[length..];
        Some((key.unwrap(), value))
    }

    fn parse_status_code(line: &str) -> Option<u16> {
        let code = line.split(' ').skip(1).next();
        match code {
            Some(status) => {
                match status.parse::<u16>() {
                    Ok(n) => return Some(n),
                    Err(_) => return None,
                }
            },
            None => return None
        };
    }
}