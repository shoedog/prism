function run(fallback) {
  const value = process.env.MODE || fallback;
  console.log(value);
}
