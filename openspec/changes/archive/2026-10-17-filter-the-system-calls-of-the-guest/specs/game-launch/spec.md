## MODIFIED Requirements

### Requirement: Confined guest process
A guest process SHALL restrict itself, before the first script runs, so that it can write,
create, remove and rename only below the game directory, can execute no file and, where the
kernel has the rule, can open no TCP connection; reading SHALL stay unrestricted. On Linux and
Android it SHALL also deny, with `EPERM`, a fixed list of system calls no game needs. Where the
kernel cannot restrict, the run SHALL say so once and go on. The host process SHALL NOT be
restricted.

#### Scenario: Write inside
- **WHEN** a script of a game run with `--guest process` writes a file below the game directory
- **THEN** the file is written

#### Scenario: Write outside
- **WHEN** a script of that game opens a file for writing in a directory beside the game directory
- **THEN** `Errno::EACCES` is raised and no file is there

#### Scenario: States
- **WHEN** a headless run with `--guest process` and `--states "40:save 0, 80:load 0"` is made with and without the restriction
- **THEN** both runs dump the same frames

#### Scenario: Denied system calls
- **WHEN** a script of a game run with `--guest process` calls `system("true")` and `Process.exec("true")`, and code in the guest opens an `AF_INET` socket
- **THEN** `system` starts nothing, `Process.exec` raises `Errno::EPERM`, and the socket fails with `EPERM`, while an `AF_UNIX` socket, file I/O, threads and `fork` work
