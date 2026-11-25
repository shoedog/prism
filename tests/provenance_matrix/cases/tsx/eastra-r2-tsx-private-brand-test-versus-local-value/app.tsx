class Mark {
  #edition = 1;
  view(candidate: object, edition: string) {
    const matches = #edition in candidate;
    return <span>{matches ? edition : ''}</span>;
  }
}
