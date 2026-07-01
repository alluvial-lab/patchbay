"use strict";
/* ----------------------------------------------------------------------------------
 * Copyright 2024 Informal Systems
 * Licensed under the Apache License, Version 2.0.
 * See LICENSE in the project root for license information.
 * --------------------------------------------------------------------------------- */
Object.defineProperty(exports, "__esModule", { value: true });
exports.compileToTlaplus = compileToTlaplus;
/**
 * Use apalache to convert quint parse data into TLA+
 *
 * @author Shon Feder, Informal Systems, 2024
 * @author Igor Konnov, konnov.phd, 2024
 *
 * @module
 */
const apalache_1 = require("./apalache");
/**
 * Get apalache to convert quint parse data into TLA+
 *
 * @param serverEndpoint
 *   a server endpoint
 *
 * @param apalacheVersion
 *  the version of Apalache to use if there is no active server connection
 *
 * @param parseDataJson the flattened, analyzed, parse data, in as a json string
 *
 * @returns right(tlaString) if parsing succeeds, or left(err) explaining the failure
 */
async function compileToTlaplus(serverEndpoint, apalacheVersion, parseDataJson, verbosityLevel) {
    const config = {
        input: {
            source: {
                type: 'string',
                format: 'qnt',
                content: parseDataJson,
            },
        },
    };
    const connectionResult = await (0, apalache_1.connect)(serverEndpoint, apalacheVersion, verbosityLevel);
    return connectionResult.asyncChain(conn => conn.tla(config));
}
//# sourceMappingURL=compileToTlaplus.js.map