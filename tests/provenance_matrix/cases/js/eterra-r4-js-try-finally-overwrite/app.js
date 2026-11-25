function run() {
  let value = process.argv[2];
  try { value = process.env.MODE; } finally { value = 'fixed'; }
  console.log(value);
}
