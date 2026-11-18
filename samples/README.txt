Sample Guitar Pro song files used to match the format.
Only song files you made or own; nothing from the Guitar Pro program folder.

Morning Walk.gp
    The demo song saved by GuitarCraft in the format Guitar Pro 8 opens.

Morning Walk Gp8.gp
    The demo song saved by Guitar Pro 8 with nothing added. Every
    "Morning Walk - <mark>.gp" file below is this song with one mark added in
    Guitar Pro 8; comparing the two shows how Guitar Pro stores that mark.
    The "(saved by GuitarCraft)" copies are the same songs saved back by
    GuitarCraft, to open in Guitar Pro 8 and check.

Empty.gp (if you put it back)
    An empty song saved by Guitar Pro 8. Some of the app's tests read it;
    without it those tests are skipped.


PALETTE MARKS AND GUITAR PRO 8 FILES
====================================

Confirmed from a Guitar Pro 8 file (GuitarCraft writes and reads them the same way):
    legato, wah open/closed, arpeggio up/down (with its duration and start
    time), golpe finger/thumb, pick scrape up/down, left- and right-hand
    fingering, timer, a bar repeated on its own (repeat signs with a count).

Written from public descriptions of the format, not yet checked with a file:
    shift slide, slide out upwards, slide in from above.
    Timer: the file only had one at the start of the song (time 0), so it
    isn't known yet whether Guitar Pro counts seconds or something finer.

Set aside for later (15 Nov 2026: the main question is answered; these
couldn't be added in Guitar Pro 8, so they wait until a test file is made).
Nothing is lost: GuitarCraft keeps them in its own hidden note in the file,
which Guitar Pro 8 ignores, so only Guitar Pro won't show them yet:
    Barre ("BV", Notation section of the palette)
    Slash notation (Notation section)
    Stem direction and beaming (the row of note icons under the Notation section)
    Free time (the dotted bar line, Bars section, next to the triplet feel)
    Repeat the previous bar / two bars (the slash-and-dots signs, Bars section)
    Ornaments (mordents, turns), artificial harmonic types (A.H., T.H., S.H.),
    left-hand tapping, sustain pedal, fade out, volume swell, whammy bar,
    bridge vibrato, rasgueado, dead slap, string number.

Tips for the hard ones (from the user guide and the palette pictures):
    - Barre, slash, stems and beaming are greyed out in Guitar Pro 8 until a
      note is selected, and stems and beaming only work while standard
      notation is shown (Track > Notation). Click a note in the notation
      staff first, then the tool.
    - Free time and the bar repeat signs act on the whole bar: click inside
      the bar (an empty bar works), then the tool.
    - Barre opens a small window to choose the fret and full or half barre.
    - If a tool still won't turn on, the same command is usually in the menus
      (Bar menu for free time and repeats, Note or Effects menu for the rest).

Check files from "--gp-tests" go to the shots folder.

Chord charts
------------
Morning Walk chart.chart    A sample chord chart (made-up words). Open it with
                            File > Open. Chord charts are saved only as .chart
                            files, never as Guitar Pro files.

Import and export (17-18 Nov 2026)
----------------------------------
Morning Walk - GP8.gpx      A .gpx saved by Guitar Pro 8: the model for GuitarCraft's own
                            .gpx export (file layout and file list).
Morning Walk.gpx            An early .gpx export of the demo song, made BEFORE the layout
                            fix (Guitar Pro 8 refused it). Use "Check GPX 1" instead, or
                            export again with File > Export > GPX.
Morning Walk.musicxml       GuitarCraft's MusicXML export. Guitar Pro 8 imports it
                            (confirmed 18 Nov).
Check GPX 1, 2, 3           Made by "guitarcraft --gpx-tests": our export; Guitar Pro 8's own
                            files in our packing; our score with Guitar Pro 8's other files.
                            All three open in Guitar Pro 8 (confirmed 18 Nov).
