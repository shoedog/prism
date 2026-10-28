class Main {
  static final class Node {
    Node next;
  }

  void unlink(Node[] left, Node[] right) {
    for (Node cursor = left[0];
        true;
        cursor = cursor.next) {
      if (cursor == null) {
        break;
      }
      cursor = cursor.next;
    }

    for (Node cursor = right[0];
        true;
        cursor = cursor.next) {
      sink(cursor);
      cursor = cursor.next;
    }
  }

  void normalize(int[] first, int[] second) {
    for (int i = 0; i < first.length; i++) {
      first[i] = flip(first[i]);
      first[i] += 1;
    }

    for (int i = 0; i < second.length; i++) {
      sink(i);
      second[i] = flip(second[i]);
      second[i] ^= 1;
    }
  }
}
