#!/bin/bash

# Script to run the frontend preview server

# Source nvm to use node
export NVM_DIR="$HOME/.nvm"
[ -s "$NVM_DIR/nvm.sh" ] && \. "$NVM_DIR/nvm.sh"

# Change to frontend directory
cd frontend

# Run the preview server
npx vite preview --port 3000 --host
