import { Controller, Get, Module, forwardRef } from '@nestjs/common';
import { NestFactory } from '@nestjs/core';

@Controller('collections')
class CollectionsController {
  @Get('featured')
  featured() {
    return [];
  }
}

@Module({
  imports: [forwardRef(() => AppModule)],
  controllers: [CollectionsController],
})
class CollectionsModule {}

@Module({ imports: [forwardRef(() => CollectionsModule)] })
class AppModule {}

export async function createApp() {
  const app = await NestFactory.create(AppModule);
  await app.init();
  return app;
}
