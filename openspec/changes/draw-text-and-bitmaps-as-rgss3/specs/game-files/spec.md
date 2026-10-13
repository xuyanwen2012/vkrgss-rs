## ADDED Requirements

### Requirement: Version of the library of RGSS3
The version of a runtime DLL SHALL be readable from the bytes of the file: the four numbers
of the file version in its version record, found by the record's signature 0xFEEF04BD
followed by the structure version 0x00010000, all little-endian. Bytes without such a record,
or that end inside it, SHALL give no version and no error. The file name SHALL NOT be used.

#### Scenario: Version 3.0.0.1
- **WHEN** bytes are read that hold, after other bytes, the words 0xFEEF04BD, 0x00010000, 0x00030000 and 0x00000001
- **THEN** the version is 3.0.0.1

#### Scenario: Version 3.0.1.1
- **WHEN** the last of those words is 0x00010001
- **THEN** the version is 3.0.1.1

#### Scenario: No record
- **WHEN** an empty file, a file of 1000 zero bytes and a file that ends 3 bytes after the signature are read
- **THEN** each gives no version

#### Scenario: Signature without the structure version
- **WHEN** the signature is followed by another word than 0x00010000 and a full record comes later in the bytes
- **THEN** the version is that of the later record
