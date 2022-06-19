<template>
    <div class="inboxDropdown">
        <v-card color="secondary">
            <v-btn color="secondary" @click="openChatDialogue()">
                Open chat dialogue
            </v-btn>
            <v-list>
                <div v-if="this.numberOfUsersWithNewMessages == 0">
                    <v-card-text
                        >Your inbox is clear... <br />
                        no new messages!</v-card-text
                    >
                </div>
                <div v-else>
                    <v-header
                        >You have messages from <br />
                        {{ this.numberOfUsersWithNewMessages }} users</v-header
                    >
                    <v-subheader>You have messages from...</v-subheader>
                </div>
                <div class="listOfUsersWithUnreadStatus">
                    <ul>
                        <div
                            v-for="user of usernamesWithUnreadMessages"
                            v-bind:key="user"
                        >
                            <router-link
                                :to="{
                                    name: 'chatDialogue',
                                    params: { userTo: user.username },
                                }"
                                style="color: black"
                            >
                                {{ user.username }}
                            </router-link>
                        </div>
                    </ul>
                </div>
            </v-list>
        </v-card>
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
export default {
    props: ['item'],

    beforeMount() {
        this.getUserIDsWithUnreadChats()
    },
    data() {
        return {
            numberOfUsersWithNewMessages: 0,
            usernamesWithUnreadMessages: [],
            url: '/internal/messages/unread',

            // error handling
            errorMessage: '',
            errorSnackbar: false,
        }
    },
    components: {},

    methods: {
        openChatDialogue() {
            this.$router.push('chatDialogue')
        },
        getUserIDsWithUnreadChats() {
            this.$http
                .get(this.url, {}, { withCredentials: true })
                .then((response) => {
                    if (response.data.length !== 0) {
                        this.usernamesWithUnreadMessages = response.data
                        this.numberOfUsersWithNewMessages = this.usernamesWithUnreadMessages.length
                    } 
                })
                .catch((error) => {
                    this.errorMessage = 'Problem contacting the server'
                    this.errorSnackbar = true
                })
        },
    },
}
</script>

<style scoped>
a {
    text-decoration: none;
}
a:hover {
    text-decoration: underline;
}

.inboxDropdown {
    min-width: 200px;
    min-height: 300px;
}

</style>
