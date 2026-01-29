# Motion Detector

A Rust-based motion detection application that works with Logitech and other USB cameras.

## Features

- Real-time motion detection using OpenCV
- Logitech camera compatibility
- Configurable sensitivity and detection area
- Automatic snapshot capture when motion is detected
- Multiple camera support
- CLI interface with customizable parameters

## Prerequisites

Install the required system dependencies:

### Ubuntu/Debian
```bash
sudo apt update
sudo apt install libopencv-dev pkg-config
```

### macOS
```bash
brew install opencv
```

### Windows
Download and install OpenCV from the official website and set the OPENCV_LINK_PATHS environment variable.

## Installation

1. Clone or create the project:
```bash
cargo build --release
```

## Usage

Basic usage:
```bash
cargo run --release
```

With custom parameters:
```bash
cargo run --release -- --device 0 --sensitivity 0.2 --min-area 300 --verbose
```

### Options

- `-d, --device <INDEX>`: Camera device index (default: 0)
- `-s, --sensitivity <VALUE>`: Motion sensitivity 0.0-1.0 (default: 0.3)
- `-m, --min-area <PIXELS>`: Minimum motion area in pixels (default: 500)
- `-v, --verbose`: Enable verbose output

### Logitech Camera Compatibility

The app automatically detects and works with Logitech cameras. Use the verbose flag to see available cameras:

```bash
cargo run --release -- --verbose
```

This will list all detected cameras with their resolutions.

## How It Works

1. Captures video frames from the camera
2. Converts frames to grayscale and applies Gaussian blur
3. Computes frame differences to detect motion
4. Uses contour detection to identify significant motion areas
5. Saves timestamped snapshots when motion exceeds thresholds
6. Prevents false positives with configurable sensitivity and minimum area

## Output

When motion is detected, the app prints:
```
[2024-01-15 14:30:25] MOTION DETECTED! (#1)
  Snapshot saved: motion_20240115_143025.jpg
```

Snapshots are saved in the current directory with timestamp filenames.