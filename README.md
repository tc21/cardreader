# cardreader
Card reader designed for ADX

# Known bugs
- Card disconnect is wonky; since it tries to switch certain cards to felica (and fails), the card disconnects before it is physically removed. Hence the 2s delay before scanning again.
