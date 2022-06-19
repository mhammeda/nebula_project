<template>
    <div class="chatDialogue">
        <v-row dense>
            <v-col :cols="3">
                <ChatDialogueSidebar v-on:changeUserTo="updateUserTo($event)" />
            </v-col>
            <v-col :cols="9">
                <ChatDialogueMessageBody
                    v-bind:userTo="this.userTo"
                    v-bind:messages="this.messages"
                    v-on:updateMessages="getMessages()"
                />
            </v-col>
        </v-row>
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
import ChatDialogueSidebar from './ChatDialogueSidebar.vue'
import ChatDialogueMessageBody from './ChatDialogueMessageBody.vue'

export default {
    mounted() {
        // grab messages on mount
        if (this.$route.params.userTo != null) {
            this.getMessages()
            this.markChatAsRead()
        }
    },
    components: {
        ChatDialogueSidebar,
        ChatDialogueMessageBody,
    },
    data() {
        return {
            valid: true,
            url: '/internal/messages/',
            userTo: this.$route.params.userTo,
            messageRules: [v => !!v || 'Message Required'],
            messages: [],

            // error handling
            errorSnackbar: false,
            errorMessage: '',
        }
    },
    methods: {
        // send put request to set messages as read
        markChatAsRead() {
            this.$http
                .put(
                    this.url + this.userTo + '/read',
                    {},
                    { withCredentials: true }
                )
                .then(response => {
                    // TODO: future code to show unread/read messages
                })
                .catch(error => {
                    this.errorMessage = 'problem setting message as read'
                    this.errorSnackbar = true
                })
        },

        getMessages() {
            this.$http
                .get(
                    this.url + this.userTo,
                    {
                        userId: this.userTo,
                    },
                    { withCredentials: true }
                )
                .then(response => {
                    console.log(response)
                    // check we got posts in response
                    if (response.data.length !== 0) {
                        this.messages = response.data
                    }
                })
                .catch(error => {
                    this.errorMessage = 'Problem contacting the server'
                    this.errorSnackbar = true
                })
        },

        updateUserTo(newUser) {
            this.messages = []
            this.userTo = newUser
            this.markChatAsRead()
            this.getMessages()
        },
    },
}
</script>
