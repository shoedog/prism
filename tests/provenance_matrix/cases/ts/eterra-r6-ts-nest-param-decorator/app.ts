import { Controller, Get, Param } from '@nestjs/common';
@Controller('users')
export class UsersController {
  @Get(':id')
  show(@Param('id') id: string) {
    return id;
  }
}
