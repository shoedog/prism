import 'reflect-metadata';
import { Controller, Patch, Module } from '@nestjs/common';
import { NestFactory } from '@nestjs/core';

@Controller('notes')
class RecordsController {
    @Patch(':id')
    updateNote() {
        return { ok: true };
    }
}

@Module({ controllers: [RecordsController] })
class AppModule {}

export async function createApp() {
    const app = await NestFactory.create(AppModule);
    return app;
}
