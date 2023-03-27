#!/usr/bin/env nu
def "get-tiles wang" [] {
    each {let wang = $in; http get $'http://www.cr31.co.uk/stagecast/art/atlas/edge/($wang).png' | save -f $'assets/wang/($wang).png'}
}
