'use strict';

// Trimmed from hapi PR #4552 (MIT-licensed) for prism regression testing.
// https://github.com/hapijs/hapi/pull/4552
// Only the function bodies relevant to the diff are retained; intervening
// lines are padded so absolute line numbers align with the diff hunks
// (line 269: stream.on('close', aborted); line 368: from.on('close', ...);
//  lines 378-383: new internals.destroyPipe function).

const internals = {};

// --- Padding lines to align function bodies with diff hunks ---
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
internals.pipe = function (request, stream) {

    const aborted = () => {
        stream.removeAllListeners();
        stream.destroy();
    };

    if (request._closed) {
        request.raw.res.removeListener('error', aborted);
        return team.work;
    }

    if (stream._readableState && stream._readableState.flowing) {
        stream.unpipe(request.raw.res);
    }
    else {
        stream.on('error', end);
        stream.on('close', aborted);
        stream.pipe(request.raw.res);
    }

    return team.work;
};


//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
internals.chain = function (sources) {

    let from = sources[0];
    for (let i = 1; i < sources.length; ++i) {
        const to = sources[i];
        if (to) {
            from.on('close', internals.destroyPipe.bind(from, to));
            from.on('error', internals.errorPipe.bind(from, to));
            from = from.pipe(to);
        }
    }

    return from;
};


internals.destroyPipe = function (to) {

    if (!this.readableEnded && !this.errored) {
        to.destroy();
    }
};

internals.errorPipe = function (to, err) {

    to.emit('error', err);
};

