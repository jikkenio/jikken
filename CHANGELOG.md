Next (Version determined when release is cut)
=====

0.2.0
=====

Bugfixes:
* Fix label when printing tests by number. They now print starting at 1 instead of 0.

Features:
* Improved test execution with invalid urls. Now properly prints out helpful error and fails the test.
* Consolidated two modes of test runs into a single code path. This simplifies expanding test_runner functionality in the future.
* Added dry-run mode. This is a new CLI argument (-d) which prints a description of the steps that will happen without actually calling the apis.

0.1.0
=====
Initial release of the Jikken CLI tool.

Features:

* basic yaml/json test file format to define api tests
* configuration file to support `continue on failure` mode and global variables
* environment variables overriding config file and global variable definition
* tests can be tagged and execution can be invoked to limit based on provided tags
* http requests (GET, POST, PUT, PATCH)
* define headers and body data
* compare result body to pre-defined
* compare status code to pre-defined
* compare two http endpoint results against each other (body and status)
* variable embeddings into tests
* variable extraction from test responses to be embedded into later run tests
* test dependencies (to force order of running tests)
* ignore/prune parts of a json response when comparing body responses
* variable types include `int`, `string`, `date`, and `datetime`
* variables can define sequences for test iterations to cycle through
* date variables support modifiers of adding/subtracting days, weeks, or months