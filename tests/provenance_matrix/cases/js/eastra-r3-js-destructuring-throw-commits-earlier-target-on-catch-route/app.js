function missing() { throw new Error('missing edition'); }
function labels() {
  let first = 'preview', second = process.env.EDITION;
  try {
    [first, second = missing()] = [process.argv[2]];
  } catch {
    return [first, second];
  }
}
