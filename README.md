# Folder Watcher

A macOS app that monitors a designated folder and automatically organizes downloaded files by type into configured destination folders.

## Features

- Monitor a folder for new files
- Automatically organize files by type (jpeg, png, gif, txt, webp, svg, other)
- Configure destination folders for each file type
- Cross-platform ready (built with Tauri)

## Development

### Prerequisites

- Node.js and npm
- Rust and Cargo
- Tauri CLI

### Setup

1. Install dependencies:
```bash
npm install
```

2. Run in development mode:
```bash
npm run tauri dev
```

3. Build for production:
```bash
npm run tauri build
```

## Usage

1. Launch the app
2. Select a folder to watch (this will be your download destination)
3. Configure destination folders for each file type
4. Click "Start Watching"
5. Any files added to the watched folder will be automatically organized

## Configuration

Configuration is stored in your system's config directory:
- macOS: `~/Library/Application Support/folder-watcher/config.json`

The configuration includes:
- `watched_folder`: Path to the folder being monitored
- `mappings`: Object mapping file extensions to destination folders




