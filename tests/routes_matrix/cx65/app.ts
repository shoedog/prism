import 'reflect-metadata';
import { Controller, Put, Module } from '@nestjs/common';
import { NestFactory } from '@nestjs/core';

@Controller('policies')
class RecordsController {
    @Put('current')
    replacePolicy() {
        return { ok: true };
    }
}

@Module({ controllers: [RecordsController] })
class AppModule {}

export async function createApp() {
    const app = await NestFactory.create(AppModule);
    return app;
}
