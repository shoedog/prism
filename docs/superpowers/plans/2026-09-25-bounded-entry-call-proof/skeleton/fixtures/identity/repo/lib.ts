export const both = (a: number, later: number) => a + later;
export const mid = (a: number, { pad }: { pad: number }, later: number) => a + pad + later;
export const fieldOnly = ({ pad }: { pad: number }, st: { x: number }) => pad + st.x;
export const bare = ({ pad }: { pad: number }, st: { x: number }) => { const s = st; return pad + s.x; };
