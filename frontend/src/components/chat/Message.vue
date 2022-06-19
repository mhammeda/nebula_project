<template>
    <div class="message">
        <div v-if="userTo == this.message.sender.username">
            <div class="recieverMessage">
                <p>
                    <b>{{ this.message.sender.username }}: </b>
                    {{ this.message.content.text.text }}
                </p>
            </div>
        </div>
        <div v-else>
            <div class="senderMessage">
                <p>
                    <b>{{ this.message.sender.username }}: </b>
                    {{ this.message.content.text.text }}
                </p>
            </div>
        </div>
    </div>
</template>

<script>
export default {
    props: ['message', 'userTo'],
    components: {},
    data() {
        return {}
    },
    methods: {},
}
</script>

<style scoped>
.message {
    width: 900px;
    height: 40px;
}

.recieverMessage {
    background-color: rgb(119, 119, 119);
    color: rgb(247, 233, 233);
}

.senderMessage {
    background-color: rgb(108, 180, 0);
    color: rgb(247, 233, 233);
}
</style>
