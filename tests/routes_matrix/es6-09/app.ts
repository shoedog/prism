import { Controller, Get, Module } from '@nestjs/common';
import type { DynamicModule } from '@nestjs/common';
import { NestFactory } from '@nestjs/core';

@Controller('bulletins')
class BulletinController {
  @Get('current')
  current() {
    return [];
  }
}

@Controller('archive')
class ArchiveController {
  @Get('bulletins')
  bulletins() {
    return [];
  }
}

@Module({})
class ArchiveModule {
  static register(enabled: boolean): DynamicModule {
    return { module: ArchiveModule, controllers: enabled ? [ArchiveController] : [] };
  }
}

export async function createApp(enableArchive: boolean) {
  @Module({
    imports: [ArchiveModule.register(enableArchive)],
    controllers: [BulletinController],
  })
  class AppModule {}

  const app = await NestFactory.create(AppModule);
  await app.init();
  return app;
}
