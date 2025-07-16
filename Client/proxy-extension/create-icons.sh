#!/bin/bash

# Create simple colored PNG icons using ImageMagick (if available) or just copy placeholders

# Check if convert command is available
if command -v convert &> /dev/null; then
    echo "Creating icons with ImageMagick..."
    
    # Create base icon (blue)
    convert -size 16x16 xc:"#3B82F6" -draw "fill white circle 8,8 11,8" public/icons/icon16.png
    convert -size 48x48 xc:"#3B82F6" -draw "fill white circle 24,24 33,24" public/icons/icon48.png  
    convert -size 128x128 xc:"#3B82F6" -draw "fill white circle 64,64 88,64" public/icons/icon128.png
    
    # Create active icons (green)
    convert -size 16x16 xc:"#10B981" -draw "fill white circle 8,8 11,8" public/icons/icon16-active.png
    convert -size 48x48 xc:"#10B981" -draw "fill white circle 24,24 33,24" public/icons/icon48-active.png
    convert -size 128x128 xc:"#10B981" -draw "fill white circle 64,64 88,64" public/icons/icon128-active.png
    
    echo "Icons created successfully!"
else
    echo "ImageMagick not found. Creating placeholder files..."
    
    # Create empty placeholder files
    touch public/icons/icon16.png
    touch public/icons/icon48.png  
    touch public/icons/icon128.png
    touch public/icons/icon16-active.png
    touch public/icons/icon48-active.png
    touch public/icons/icon128-active.png
    
    echo "Placeholder icon files created. Please replace with actual PNG icons."
fi
