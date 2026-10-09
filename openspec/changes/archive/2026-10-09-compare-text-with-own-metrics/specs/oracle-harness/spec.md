# Spec Delta

## ADDED Requirements

### Requirement: Own metrics with shared fonts
When fonts are shared with `--font`, the harness SHALL run vkxp with fonts' own metrics, so
that both runtimes size the shared font by its own line box.

#### Scenario: Command line
- **WHEN** the harness builds vkxp's command line for a run with `--font`
- **THEN** it contains `--font-own-metrics`, and without `--font` it does not
