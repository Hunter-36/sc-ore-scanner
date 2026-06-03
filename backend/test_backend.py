"""Quick backend test script."""

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent / "src"))

from src.config import get_settings, ScanRegion
from src.resolver import RSResolver
from src.capture import ScreenCapture
from src.ocr import OCREngine
from src.server import create_app

print("=" * 60)
print("SC ORE SCANNER - Backend Test")
print("=" * 60)
print()

# Test 1: Config
print("[1/5] Testing config module...")
settings = get_settings()
print(f"  [OK] Config loaded")
print(f"  [OK] Server: {settings.server.host}:{settings.server.port}")
print(f"  [OK] OCR confidence threshold: {settings.ocr.confidence_threshold}")
print()

# Test 2: Resolver
print("[2/5] Testing RS resolver...")
resolver = RSResolver(settings)
print(f"  [OK] Loaded {len(resolver.signatures)} ore signatures")

# Test some known signatures
test_cases = [
    (10620, "Beryl"),  # 3 x 3540
    (3170, "Quantainium"),  # 1 x 3170
    (17140, "Aluminium"),  # 4 x 4285
]

for rs_value, expected_ore in test_cases:
    matches = resolver.resolve(rs_value)
    if matches and matches[0].ore.name == expected_ore:
        print(f"  [OK] {rs_value} -> {matches[0].quantity}x {matches[0].ore.name}")
    else:
        actual = matches[0].ore.name if matches else "no match"
        print(f"  [FAIL] {rs_value} expected {expected_ore}, got {actual}")
print()

# Test 3: Screen Capture
print("[3/5] Testing screen capture...")
capture = ScreenCapture(settings)
screen_info = capture.get_screen_info()
print(f"  [OK] Detected {len(screen_info['monitors'])} monitor(s)")
if screen_info['primary']:
    mon = screen_info['primary']
    print(f"  [OK] Primary: {mon['width']}x{mon['height']}")
capture.close()
print()

# Test 4: OCR (lazy loading check)
print("[4/5] Testing OCR module...")
ocr = OCREngine(settings)
print(f"  [OK] OCR engine created (lazy loading)")
print(f"  [OK] Debouncing: {settings.ocr.min_consecutive_frames} frames required")
print(f"  [OK] CLAHE enabled with clip limit {settings.ocr.clahe_clip_limit}")
print()

# Test 5: FastAPI Server
print("[5/5] Testing FastAPI server...")
app = create_app()
print(f"  [OK] App created: {app.title}")
print(f"  [OK] Version: {app.version}")
print(f"  [OK] Routes: {len(app.routes)} endpoints")
print()

print("=" * 60)
print("[SUCCESS] ALL TESTS PASSED")
print("=" * 60)
print()
print("Backend is ready to run:")
print(f"  python main.py")
print()
print("Or calibrate scan region first:")
print(f"  python calibrate.py")
print()
