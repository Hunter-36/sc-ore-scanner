============================================================
  SC ORE SCANNER  -  Read Me
============================================================

A real-time overlay for Star Citizen mining. It reads the RS
(Radar Signature) number off your mining scanner HUD and shows
which ore it is and how many nodes - no typing, no alt-tabbing.

------------------------------------------------------------
  REQUIREMENTS
------------------------------------------------------------
- Windows 10 or 11
- An internet connection for the one-time setup
- ~300 MB free disk space

You do NOT need to install Python, Node, or anything else
yourself - setup.bat handles it.

------------------------------------------------------------
  INSTALL  (one time)
------------------------------------------------------------
1. Unzip this folder anywhere (e.g. your Desktop).
2. Double-click  setup.bat
   - It installs the backend and downloads ~150 MB.
   - Then it opens a calibration window: click-drag a box
     over the spot on your mining HUD where the RS number
     ("10,620"-style teal text) appears, and release.

------------------------------------------------------------
  RUN  (every time you play)
------------------------------------------------------------
1. Launch Star Citizen and get in your mining ship / scanner.
2. Double-click  launch.bat
   - A backend window opens, then the overlay appears top-right.
   - Status goes OFFLINE -> READY -> SCANNING.
3. Scan an asteroid. Within ~2 seconds the overlay shows the
   ore name and quantity (e.g. "Beryl  3x"), color-coded by tier.

To stop: close the backend window and the overlay.

------------------------------------------------------------
  "Windows protected your PC"  (SmartScreen)
------------------------------------------------------------
Because this is a free community tool and isn't code-signed
(signing certificates are expensive), Windows may warn you when
you run the .bat or the overlay. This is expected for unsigned
apps. To proceed:

   Click "More info"  ->  "Run anyway"

Everything here is open source - you can read every script and
build it yourself from the source if you prefer:
   https://github.com/Hunter-36/sc-ore-scanner

------------------------------------------------------------
  TROUBLESHOOTING
------------------------------------------------------------
- Overlay stuck on OFFLINE: the backend is still loading; wait
  ~10s. If it never connects, make sure the backend window is
  open and didn't show an error.
- Nothing detected while scanning: re-run setup.bat to
  recalibrate the scan region (especially if you changed your
  screen resolution or HUD scale).
- Still stuck? Open an issue on GitHub (link above).

------------------------------------------------------------
  SUPPORT THE PROJECT
------------------------------------------------------------
This is built and maintained for free, on personal time. If it
saves you some aUEC and you'd like to help keep it updated as
Star Citizen changes, you can buy me a coffee:

   https://ko-fi.com/huntersutton36

Never required - starring/sharing the repo helps too. Thank you! o7
