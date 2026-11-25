function run(flag) {
  let value = process.argv[2];
  if (flag) value = "left"; else value = "right";
  console.log(value);
}
