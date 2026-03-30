const fs = require('fs');
const path = require('path');
const folderName = process.argv[2];

function listContents(dir, indent = '') {
  const items = fs.readdirSync(dir);
  items.forEach(item => {
    const fullPath = path.join(dir, item);
    const isDir = fs.statSync(fullPath).isDirectory();
    console.log(`${indent}${isDir ? 'folder named ' : ''}${item}`);
    if (isDir) {
      listContents(fullPath, indent + '  ');
    }
  });
}

listContents(folderName);