#!/usr/bin/env node

/**
 * Setup script for Jujutsu (jj) version control
 * 
 * This script helps users:
 * - Check if jj is installed
 * - Initialize jj in their repository
 * - Set up a default jj configuration
 */

const { execSync } = require('child_process');
const fs = require('fs');
const path = require('path');
const os = require('os');

const colors = {
  reset: '\x1b[0m',
  green: '\x1b[32m',
  yellow: '\x1b[33m',
  red: '\x1b[31m',
  blue: '\x1b[34m',
  bold: '\x1b[1m'
};

function log(message, color = colors.reset) {
  console.log(`${color}${message}${colors.reset}`);
}

function checkJjInstalled() {
  try {
    const version = execSync('jj --version', { encoding: 'utf8', stdio: ['pipe', 'pipe', 'pipe'] });
    log(`✓ Jujutsu is installed: ${version.trim()}`, colors.green);
    return true;
  } catch (error) {
    return false;
  }
}

function showInstallInstructions() {
  log('\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━', colors.blue);
  log('  Jujutsu (jj) Installation Instructions', colors.bold);
  log('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━', colors.blue);
  log('\nJujutsu (jj) is not installed on your system.\n');
  log('Please install it using one of the following methods:\n');
  
  const platform = os.platform();
  
  if (platform === 'darwin') {
    log('  • Homebrew:', colors.green);
    log('    brew install jj\n');
  } else if (platform === 'linux') {
    log('  • Homebrew (Linux):', colors.green);
    log('    brew install jj\n');
    log('  • Arch Linux:', colors.green);
    log('    pacman -S jj\n');
  }
  
  log('  • Cargo (all platforms):', colors.green);
  log('    cargo install --locked jj-cli\n');
  
  log('  • Binary downloads:', colors.green);
  log('    https://github.com/martinvonz/jj/releases\n');
  
  log('For more information, visit:', colors.blue);
  log('  https://martinvonz.github.io/jj/\n');
  log('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n', colors.blue);
}

function isJjRepo(repoPath) {
  const jjDir = path.join(repoPath, '.jj');
  return fs.existsSync(jjDir) && fs.statSync(jjDir).isDirectory();
}

function initJjRepo(repoPath) {
  try {
    log(`\nInitializing jj repository in: ${repoPath}`, colors.blue);
    execSync('jj init --git', { cwd: repoPath, stdio: 'inherit' });
    log('✓ Successfully initialized jj repository!', colors.green);
    return true;
  } catch (error) {
    log(`✗ Failed to initialize jj repository: ${error.message}`, colors.red);
    return false;
  }
}

function getDefaultJjConfig() {
  return `# Jujutsu configuration for Vibe Kanban
# For more information: https://martinvonz.github.io/jj/latest/config/

[user]
# name = "Your Name"
# email = "your.email@example.com"

[ui]
# Use the default diff tool
diff-editor = "diff"
# Show relative timestamps in logs
relative-timestamps = true
# Enable colored output
color = "auto"

[git]
# Automatically push branches to git remote
push-branch-prefix = ""
# Fetch from all remotes by default
auto-local-branch = true

[revsets]
# Log shows local branches plus main and master
log = "@ | branches() | (main | master)"

[aliases]
# Common shortcuts
st = ["status"]
l = ["log"]
show = ["show"]
`;
}

function setupJjConfig() {
  const configDir = path.join(os.homedir(), '.config', 'jj');
  const configPath = path.join(configDir, 'config.toml');
  
  // Check if config already exists
  if (fs.existsSync(configPath)) {
    log(`\n✓ jj config already exists at: ${configPath}`, colors.green);
    return true;
  }
  
  try {
    // Create config directory if it doesn't exist
    if (!fs.existsSync(configDir)) {
      fs.mkdirSync(configDir, { recursive: true });
    }
    
    // Write default config
    fs.writeFileSync(configPath, getDefaultJjConfig(), 'utf8');
    log(`\n✓ Created default jj config at: ${configPath}`, colors.green);
    log('  You may want to edit this file to set your name and email.', colors.yellow);
    return true;
  } catch (error) {
    log(`\n✗ Failed to create jj config: ${error.message}`, colors.red);
    return false;
  }
}

function main() {
  log('\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━', colors.blue);
  log('  Jujutsu (jj) Setup for Vibe Kanban', colors.bold);
  log('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━', colors.blue);
  
  // Check if jj is installed
  if (!checkJjInstalled()) {
    showInstallInstructions();
    process.exit(1);
  }
  
  // Setup jj config
  setupJjConfig();
  
  // Check if we should initialize a repo
  const repoPath = process.argv[2] || process.cwd();
  
  if (!fs.existsSync(repoPath)) {
    log(`\n✗ Directory does not exist: ${repoPath}`, colors.red);
    process.exit(1);
  }
  
  if (isJjRepo(repoPath)) {
    log(`\n✓ Directory is already a jj repository: ${repoPath}`, colors.green);
  } else {
    // Ask if user wants to initialize
    const gitDir = path.join(repoPath, '.git');
    if (fs.existsSync(gitDir)) {
      log(`\nFound git repository at: ${repoPath}`, colors.yellow);
      log('Would you like to initialize jj? (This will coexist with git)', colors.yellow);
      
      // For non-interactive use, just show instructions
      log('\nTo initialize jj in this repository, run:', colors.blue);
      log(`  cd ${repoPath}`, colors.green);
      log('  jj init --git\n', colors.green);
    } else {
      log(`\nDirectory is not a git or jj repository: ${repoPath}`, colors.yellow);
      log('To initialize jj, first initialize git, then run:', colors.blue);
      log('  git init', colors.green);
      log('  jj init --git\n', colors.green);
    }
  }
  
  log('\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━', colors.blue);
  log('  Setup complete!', colors.bold + colors.green);
  log('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n', colors.blue);
  
  log('Next steps:', colors.blue);
  log('  1. Edit your jj config to set your name and email:', colors.reset);
  log('     ~/.config/jj/config.toml\n', colors.green);
  log('  2. Learn more about jj:', colors.reset);
  log('     https://martinvonz.github.io/jj/latest/tutorial/\n', colors.green);
  log('  3. Start using jj with Vibe Kanban!', colors.reset);
  log('     npx vibe-kanban\n', colors.green);
}

if (require.main === module) {
  main();
}

module.exports = {
  checkJjInstalled,
  isJjRepo,
  initJjRepo,
  setupJjConfig,
  getDefaultJjConfig
};
