// Test npm package resolution with lodash
import _ from 'lodash';

export function doubled(arr) {
    return _.map(arr, x => x * 2);
}

export function saveData(data) {
    return _.omit(data, ['internal', 'debug']);
}

// Test some other lodash functions
export function chunkedData(arr, size) {
    return _.chunk(arr, size);
}

console.log('Using lodash version:', _.VERSION || 'unknown');