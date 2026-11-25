function run(stop) {
  let value = process.env.MODE;
  if (stop) { value = process.argv[2]; throw new Error(); }
  console.log(value);
}
