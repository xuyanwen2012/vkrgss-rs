# What the prelude has for a game of RGSS1 only. Written for vkrgss.

# Raised by the original runtime's hang watchdog, which vkrgss does not have. RGSS2 has no
# such constant.
class Hangup < Exception
end
