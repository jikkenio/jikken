# jikken
A high performance REST API testing toolkit that is source control friendly. This project is currently in early beta.

# test definition format

The definition file is a YAML/JSON file with the following layout:

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

It supports global variable replacements (denoted by being wrapped with `#`) as defined in the config file. Local variables are replaced by wrapping their names with `$`.
A (hopefully) working example:

```
name: Usage Report
iterate: 2
request:
  method: Get
  url: #newUrl#/reports/v1/usage
  params:
    - param: from
      value: $startDate$
    - param: to
      value: $endDate$
    - param: userId
      value: $userId$
    - param: userType
      value: $userType$
  headers:
    - header: Authorization
      value: #authToken#
compare:
  method: Get
  url: #oldUrl#/reports/v1/usage
  params:
    - param: from
      value: $startDate$
    - param: to
      value: $endDate$
    - param: userId
      value: $userId$
    - param: userType
      value: $userType$
  headers:
    - header: Authorization
      value: #authToken#
variables:
  - name: startDate
    dataType: Date
    value: #TODAY#
    modifier:
      operation: subtract
      value: 3
      unit: days
  - name: endDate
    dataType: Date
    value: #TODAY#
    modifier:
      operation: subtract
      value: 2
      unit: days
  - name: userId
    value: [-1, 87]
    dataType: Int
  - name: userType
    value: ['AllUser', 'Alias']
    dataType: String
```

# config file format

```
[settings]
continueOnFailure=true

[globals]
newUrl="https://localhost:5001"
oldUrl="https://localhost:5002"
```