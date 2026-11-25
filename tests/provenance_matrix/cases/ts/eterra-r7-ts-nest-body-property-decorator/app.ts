import { Controller, Post, Body } from '@nestjs/common';
@Controller('posts')
export class PostsController {
  @Post()
  create(@Body('title') title: string) {
    return title;
  }
}
