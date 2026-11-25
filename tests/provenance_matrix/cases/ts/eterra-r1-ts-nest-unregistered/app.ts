import { Query } from '@nestjs/common';
class Plain {
  read(@Query('q') q: string) {
    return q;
  }
}
