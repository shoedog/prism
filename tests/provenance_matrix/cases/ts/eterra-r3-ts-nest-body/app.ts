import { Controller, Post, Body } from '@nestjs/common';
@Controller('items')
class ItemsController {
  @Post()
  create(@Body('name') name: string) {
    return name;
  }
}
