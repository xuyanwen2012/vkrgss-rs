# What vkxp prepares over Ruby 1.8 only, before `prelude.rb`. Ruby 1.9 ignores `$KCODE`, and
# its load path is the one the original runtime of RGSS3 has.

$KCODE = "U"
$:.clear
