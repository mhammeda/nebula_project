<template>
    <div class="chatDialogueMessageBody">
        <h1>{{ this.userTo }}</h1>
        <div v-if="userTo == null">Welcome to the Chat Dialogue!</div>
        <div v-else-if="messages.length == 0">
            No messages with {{ this.userTo }} to show
        </div>
        <div
            class="chatDialogueMessageBodyMessages"
            ref="chatDialogueMessageBodyMessages"
        >
            <div v-for="message in messages" :key="message.id">
                <Message v-bind:message="message" v-bind:userTo="userTo" />
            </div>
        </div>
        <div class="chatDialogueSubmitSection">
            <div v-if="userTo != null">
                <v-text-field
                    :rules="messageRules"
                    outlined
                    required
                    v-model="messageToSend"
                    label="Message"
                    @keydown.enter="sendMessage"
                ></v-text-field>
                <v-btn
                    :disabled="!messageToSend"
                    color="primary"
                    @click="sendMessage"
                >
                    Send
                </v-btn>
            </div>
        </div>
        <v-snackbar v-model="errorSnackbar">
            {{ errorMessage }}

            <template v-slot:action="{ attrs }">
                <v-btn
                    color="red"
                    text
                    v-bind="attrs"
                    @click="errorSnackbar = false"
                >
                    Close
                </v-btn>
            </template>
        </v-snackbar>
    </div>
</template>

<script>
import Message from './Message.vue'

export default {
    props: ['userTo', 'messages'],
    mounted() {
        // open WebSocket connection
        const wsUri =
            (window.location.protocol === 'https:' ? 'wss://' : 'ws://') +
            window.location.host +
            '/ws/' +
            localStorage.getItem('auth')

        this.connection = new WebSocket(wsUri)

        this.connection.onmessage = event => {
            console.log(JSON.parse(event.data))
            this.$emit('updateMessages')
        }
    },
    data() {
        return {
            messageToSend: null,
            url: '/internal/messages/',

            // error handling
            errorSnackbar: false,
            errorMessage: '',

            // WebSocket connection
            connection: null,
        }
    },

    components: {
        Message,
    },

    methods: {
        sendMessage() {
            // check message to send (can't send empty message)
            if (this.messageToSend != '') {
                if (this.connection == null) {
                    // send the message
                    this.$http
                        .post(
                            this.url + this.userTo,
                            {
                                title: '',
                                content: {
                                    text: { text: this.messageToSend },
                                },
                            },
                            { withCredentials: true }
                        )
                        .then(response => {
                            // tell parent component to refresh messages
                            this.$emit('updateMessages')
                            // reset errors messages
                            this.errorMessage = ''
                            this.messageToSend = ''
                        })
                        .catch(error => {
                            // setup and show snackbar
                            this.errorMessage =
                                'there was a problem with this request.'
                            this.errorSnackbar = true
                        })
                } else {
                    this.connection.send(
                        JSON.stringify({
                            sender: {
                                username: localStorage.getItem('userid'),
                                host: window.location.hostname,
                            },
                            receiver: {
                                username: this.userTo,
                                host: window.location.hostname,
                            },
                            content: {
                                text: { text: this.messageToSend },
                            },
                        })
                    )

                    this.$emit('updateMessages')
                }
            }
        },
    },
}
</script>

<style scoped>
.chatDialogueMessageBodyMessages {
    overflow-y: scroll;
    margin-bottom: 1px;
    width: 1000px;
    height: 700px;
}

.chatDialogueSubmitSection {
    width: 1000px;
}
</style>
