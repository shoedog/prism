void consume(const char*);
void run() {
  const char* text = R"(std::getenv("MODE"))";
  consume(text);
}
