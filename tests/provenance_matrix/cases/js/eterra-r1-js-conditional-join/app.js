function run(flag) {
  let value = 'fixed';
  if (flag) value = process.env.MODE;
  console.log(value);
}
