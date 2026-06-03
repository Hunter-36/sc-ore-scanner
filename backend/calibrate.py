"""Interactive scan region calibration tool.

Usage:
    python calibrate.py

Creates a fullscreen overlay for selecting the scan region.
"""

import sys
import tkinter as tk
from pathlib import Path

# Add src to path
sys.path.insert(0, str(Path(__file__).parent / "src"))

from src.config import get_settings, ScanRegion


class RegionSelector:
    """Interactive scan region selector with fullscreen overlay."""

    def __init__(self):
        self.root = tk.Tk()
        self.root.attributes('-fullscreen', True)
        self.root.attributes('-alpha', 0.3)  # Semi-transparent
        self.root.attributes('-topmost', True)
        self.root.configure(bg='black')

        # Get screen dimensions
        self.screen_width = self.root.winfo_screenwidth()
        self.screen_height = self.root.winfo_screenheight()

        # Create canvas
        self.canvas = tk.Canvas(
            self.root,
            width=self.screen_width,
            height=self.screen_height,
            bg='black',
            highlightthickness=0
        )
        self.canvas.pack()

        # Selection state
        self.start_x = None
        self.start_y = None
        self.rect = None
        self.selected_region = None

        # Instructions
        self.instructions = self.canvas.create_text(
            self.screen_width // 2,
            50,
            text="Click and drag to select scan region\nPress ESC to cancel",
            font=('Arial', 24, 'bold'),
            fill='white'
        )

        # Bind events
        self.canvas.bind('<Button-1>', self.on_mouse_down)
        self.canvas.bind('<B1-Motion>', self.on_mouse_drag)
        self.canvas.bind('<ButtonRelease-1>', self.on_mouse_up)
        self.root.bind('<Escape>', lambda e: self.root.quit())

    def on_mouse_down(self, event):
        """Handle mouse button press."""
        self.start_x = event.x
        self.start_y = event.y

        # Create rectangle
        if self.rect:
            self.canvas.delete(self.rect)

        self.rect = self.canvas.create_rectangle(
            self.start_x, self.start_y, self.start_x, self.start_y,
            outline='cyan',
            width=3,
            fill='cyan',
            stipple='gray50'
        )

    def on_mouse_drag(self, event):
        """Handle mouse drag."""
        if self.rect:
            self.canvas.coords(
                self.rect,
                self.start_x, self.start_y,
                event.x, event.y
            )

    def on_mouse_up(self, event):
        """Handle mouse button release."""
        if self.start_x is None or self.start_y is None:
            return

        # Calculate region
        x1 = min(self.start_x, event.x)
        y1 = min(self.start_y, event.y)
        x2 = max(self.start_x, event.x)
        y2 = max(self.start_y, event.y)

        width = x2 - x1
        height = y2 - y1

        # Validate minimum size
        if width < 50 or height < 50:
            print("Region too small! Minimum 50x50 pixels")
            if self.rect:
                self.canvas.delete(self.rect)
            self.start_x = None
            self.start_y = None
            return

        self.selected_region = ScanRegion(
            x=x1,
            y=y1,
            width=width,
            height=height
        )

        print(f"\nSelected region: {width}x{height} at ({x1}, {y1})")
        print("Saving configuration...")

        # Save and exit
        self.root.quit()

    def run(self):
        """Run the selector."""
        print("=" * 60)
        print("SC ORE SCANNER - Region Calibration")
        print("=" * 60)
        print("\nInstructions:")
        print("  1. Click and drag to select the scan region")
        print("  2. Select the area where RS numbers appear in-game")
        print("  3. Release to confirm selection")
        print("  4. Press ESC to cancel")
        print("\nStarting overlay...")

        self.root.mainloop()
        self.root.destroy()

        return self.selected_region


def main():
    """Run calibration tool."""
    # Create selector
    selector = RegionSelector()
    region = selector.run()

    if region:
        # Save configuration
        settings = get_settings()
        settings.scan_region = region
        settings.save_user_config()

        print("\n" + "=" * 60)
        print("SUCCESS - Configuration saved!")
        print("=" * 60)
        print(f"\nScan Region:")
        print(f"  Position: ({region.x}, {region.y})")
        print(f"  Size: {region.width}x{region.height}")
        print(f"\nConfig file: {settings.config_file}")
        print("\nYou can now run the backend:")
        print("  python main.py")
        print()
    else:
        print("\nCalibration cancelled.")
        sys.exit(1)


if __name__ == "__main__":
    main()
