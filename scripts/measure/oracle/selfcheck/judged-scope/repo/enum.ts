const A = 10;
enum E {
  A = 1,
  B = ({
    A
  }).A,
}
const plain = {
  A
};
export {};
