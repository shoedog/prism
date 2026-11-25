function visit(choose, again) {
  let edition;
  do {
    if (choose()) {
      edition = process.argv[2];
      continue;
    }
    edition = process.env.EDITION;
  } while (again(edition));
}
