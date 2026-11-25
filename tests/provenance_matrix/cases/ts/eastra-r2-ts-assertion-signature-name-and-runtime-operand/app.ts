function assertEdition(
  edition: unknown,
): asserts edition is string {
  if (typeof edition !== 'string') throw new Error();
}
