# jikken
A high performance REST API testing toolkit that is source control friendly.

# test definition format

```
name: 
request:
    method:
    url:
    params:
      - key:
        value:
    headers:
      - key:
        value:
    body: {

    }
    
compare:
    method:
    url:
    params: 
      - key:
        value:
    headers:
      - key:
        value:
    body: {

    }
    
response: 
    status:
    headers:
      - key:
        value:
    body: {

    }
    ignore:
        - 

variables:
  - name: 
    dataType:
    value: 
    range:
        min:
        max:
    modifier:
        operation: 
        value: 
        unit: 
    format:
```
