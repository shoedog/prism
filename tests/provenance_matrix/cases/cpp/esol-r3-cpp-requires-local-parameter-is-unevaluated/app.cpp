template <class Stream>
concept Readable = requires(Stream candidate) {
    candidate.read();
};
