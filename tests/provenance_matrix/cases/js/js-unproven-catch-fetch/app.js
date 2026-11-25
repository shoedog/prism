function read() {
  try {
    throw () => 7;
  } catch (fetch) {
    const value = fetch("https://example.invalid");
    return value;
  }
}
