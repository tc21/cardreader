# cardreader
Card reader designed for ADX.
- Written for Sony RC-S380, tested on Windows 11.
- Writes scanned ID to file, then invokes scanning of that file by pressing a configured key (using segatools virtual aime to pass it on).

# Known bugs
- Card disconnect is wonky; since it tries to switch certain cards to felica (and fails), the card disconnects before it is physically removed. Hence the 2s delay before scanning again.
