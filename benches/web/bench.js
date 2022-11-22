const axios = require('axios')
const { default: PQueue } = require('p-queue');


async function main() {
    const col_range = 30
    const row_range = 20
    const level = 18
    const queue = new PQueue({ concurrency: 50 })
    const res = await axios.get("http://localhost:8080/slide_1.dzi")
    console.log(res.data)
    const code = `${col_range * row_range} total requests`
    console.time(code)
    for (let row = 0; row < row_range; row++) {
        for (let col = 0; col < col_range; col++) {
            queue.add(async () => {
                try {
                    const url = `http://localhost:8080/slide_1_files/${level}/${col}_${row}.jpg`
                    const resp = await axios.get(url)
                    return resp.data.length
                } catch (e) {
                    console.error(e)
                }
            })
        }
    }
    await queue.onIdle()
    console.timeEnd(code)
}

main()
