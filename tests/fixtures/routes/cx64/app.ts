import 'reflect-metadata';
import { Controller, Post, Module } from '@nestjs/common';
import { NestFactory } from '@nestjs/core';

@Controller('jobs')
class RecordsController {
    @Post('queue')
    queueJob() {
        return { ok: true };
    }
}

@Module({ controllers: [RecordsController] })
class AppModule {}

export async function createApp() {
    const app = await NestFactory.create(AppModule);
    return app;
}
