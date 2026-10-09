# Spec Delta

## ADDED Requirements

### Requirement: Archives with large identifiers
The harness SHALL read script archives whose entries have identifiers stored as Bignums and
SHALL write them back unchanged.

#### Scenario: Bignum identifier
- **WHEN** an archive whose second entry has the identifier 3000000000 is read, a probe is added and the result is read again
- **THEN** the entries after the probe are byte for byte as before
