/nearcore/tools/indexer/example

provide a api
curl -X GET http://127.0.0.1:6526/block/{number} | jq .
provide 10000 block, new block data will cover old data

import { types } from 'near-lake-framework';

types.StreamerMessage
