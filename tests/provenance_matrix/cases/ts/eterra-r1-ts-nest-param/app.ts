import { Controller, Get, Param } from '@nestjs/common';
@Controller('items')
class Items {
  @Get(':id')
  getOne(@Param('id') id: string) {
    return id;
  }
}
