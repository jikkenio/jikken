#![allow(dead_code)]
use super::test_descriptor::TestDescriptor;
use hyper::body;
use hyper::Client;
use hyper_tls::HttpsConnector;

pub struct TestRunner {
    run: u16,
    passed: u16,
    failed: u16
}

impl TestRunner {
    pub fn new() -> TestRunner {
        TestRunner {run: 0, passed: 0, failed: 0}
    }

    pub async fn run(&self, td: TestDescriptor, count: usize) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        println!("running test: {}", td.name.clone().unwrap_or(format!("{}", count)));
        
        if td.is_comparison {
            let result = TestRunner::validate_td_comparison_mode(td).await?;
            let label = if result { "PASSED" } else { "FAILED "};
            println!("The test {}!", label);
        } else {
            let result = TestRunner::validate_td(td).await?;
            let label = if result { "PASSED" } else { "FAILED "};
            println!("The test {}!", label);
        }

        Ok(())
    }

    async fn validate_td(td: TestDescriptor) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let uri = td.url.unwrap();
        let client = Client::builder().build::<_, hyper::Body>(HttpsConnector::new());
        let resp = client.get(uri).await?;
        
        let mut pass = true;

        let status_test = match td.status_code {
            Some(code) => TestRunner::validate_status_code(resp.status(), code),
            None => true // none defined, skip this step
        };

        pass &= status_test;
          
        // for (name, value) in resp.headers() {
        //     println!("Header: {} -> {}", name, value.to_str().unwrap());
        // }
        // let data = body::to_bytes(resp.into_body()).await?;
        // let data_compare = body::to_bytes(resp_compare.into_body()).await?;
        // pass &= data == data_compare;
        // let data_str = String::from_utf8(data.to_vec());
        // println!("Body: {}", data_str.unwrap_or(String::from("unable to load body data")));
        
        Ok(pass)
    }

    async fn validate_td_comparison_mode(td: TestDescriptor) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let uri = td.url.unwrap();
        let uri_compare = td.url_secondary.unwrap();
        let client = Client::builder().build::<_, hyper::Body>(HttpsConnector::new());
        let resp = client.get(uri).await?;
        let resp_compare = client.get(uri_compare).await?;

        let mut pass = true;
        let status_test = TestRunner::validate_status_codes(resp.status(), resp_compare.status());

        pass &= status_test;
          
        // for (name, value) in resp.headers() {
        //     println!("Header: {} -> {}", name, value.to_str().unwrap());
        // }

        let data = body::to_bytes(resp.into_body()).await?;
        let data_compare = body::to_bytes(resp_compare.into_body()).await?;

        pass &= data == data_compare;


        // let data_str = String::from_utf8(data.to_vec());

        // println!("Body: {}", data_str.unwrap_or(String::from("unable to load body data")));
        
        Ok(pass)
    }

    fn validate_status_codes(actual: hyper::StatusCode, expected: hyper::StatusCode) -> bool {
        let result = actual == expected;
        let label = if result { "PASS" } else { "FAIL" };
        if label == "FAIL" {
            println!("Expected: {}, Actual: {}", expected.as_u16(), actual.as_u16());
        }
        println!("validating status code: {}", label);
        return result;
    }

    fn validate_status_code(actual: hyper::StatusCode, expected: u16) -> bool {
        let result = actual.as_u16() == expected;
        let label = if result { "PASS" } else { "FAIL" };
        if label == "FAIL" {
            println!("Expected: {}, Actual: {}", expected, actual.as_u16());
        }
        println!("validating status code: {}", label);
        return result;
    }
}