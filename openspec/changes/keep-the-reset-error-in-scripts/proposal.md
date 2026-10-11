# Keep the reset error visible to the scripts that run again

## Why

Pressing F12 in XP-MBBS-7.0 ends the game with "Script 'EFS_Hp_Mp_Bar' line 66:
SystemStackError occurred. stack level too deep". The script guards its alias with a common
idiom of RPG Maker XP scripts:

    alias hzhj_old_dispose_make_event_param_bar dispose unless $@

In the original runtime the `Reset` exception that ended the previous run is still the current
error while the scripts are evaluated again, so `$@` is its backtrace and the alias is not
made a second time. vkxp clears the error, the alias is made again and now names the method
that calls it.

Measured on the original (RGSS103J under Wine, F12 injected with `keybd_event`): before the
first run `$!` and `$@` are nil; in the second and third run `$!` is the `Reset` exception and
`$@` is the backtrace of the `Graphics.update` that raised it. Probe and output:
`ref/tools/oracle/measurements/reset-errinfo-probe.rb`, `reset-errinfo-out.txt` (outside
this repository).

## What Changes

- After a reset, the scripts are evaluated again with the `Reset` exception as the current
  error.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `rgss-runtime`: the current error during the runs after a reset.

## Impact

`vkxp-ruby-sys` (the protected call), `vkxp-binding` (the loop over the scripts), README if it
describes the reset.
