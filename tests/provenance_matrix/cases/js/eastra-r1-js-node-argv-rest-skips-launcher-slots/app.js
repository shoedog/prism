const process = require('node:process');
if (require.main === module) {
  const [runtimePath, entryFile, ...choices] = process.argv;
  void [runtimePath, entryFile, choices];
}
