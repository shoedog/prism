import { applyDecorators, Controller, Get, Module } from '@nestjs/common';
import { NestFactory } from '@nestjs/core';

function LibraryGet(path: string) {
  return applyDecorators(Get(path));
}

@Controller('library')
class LibraryController {
  @Get('info')
  info() {
    return { available: true };
  }

  @LibraryGet('catalog')
  catalog() {
    return [];
  }
}

@Module({ controllers: [LibraryController] })
export class AppModule {}

export async function createApp() {
  const app = await NestFactory.create(AppModule);
  await app.init();
  return app;
}
