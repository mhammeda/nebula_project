<template>
    <div class="chatDialogueSidebar">
        <div class="searchBar">
            <v-text-field
                v-model="usernameSearchTerm"
                label="Username to search for"
                :rules="searchboxRules"
                @keydown.enter="searchForUser"
            >
            </v-text-field>
            <v-btn color="secondary" @click="searchForUser()"> Search </v-btn>
        </div>
        <div class="resultsBox">
            <v-card>
                <v-list>
                    <div v-if="usernamesReturned.length > 0">
                        <v-list-item
                            v-for="username in usernamesReturned"
                            :key="username"
                        >
                            <p v-on:click="updateUserTo(username)">
                                {{ username }}
                            </p>
                        </v-list-item>
                    </div>
                    <div v-else-if="usernameSearchTerm == null">
                        Search for a user you want to speak to above
                    </div>
                    <div v-else>
                        <p class="font-weight-thin">No users returned</p>
                    </div>
                </v-list>
            </v-card>
        </div>
        <div class="resultsBox">
            <h3>Existing chats</h3>

            <v-card>
                <v-list>
                    <v-list-item
                        v-for="username in existingChats"
                        :key="username"
                    >
                        <p v-on:click="updateUserTo(username)">
                            {{ username }}
                        </p>
                    </v-list-item>
                </v-list>
            </v-card>
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
export default {
    data() {
        return {
            url: '/internal/users/search/',
            usernameSearchTerm: '',
            searchboxRules: [v => !!v || 'Search term required'],
            usernamesReturned: [],

            //
            existingChats: [],

            // error handling
            errorMessage: '',
            errorSnackbar: false,
        }
    },
    components: {},
    methods: {
        searchForUser() {
            if (this.usernameSearchTerm.length === 0) {
                return
            } else {
                // send request for users matching username
                this.$http
                    .get(
                        this.url + this.usernameSearchTerm,
                        {},
                        { withCredentials: true }
                    )
                    .then(response => {
                        this.usernamesReturned = []
                        if (response.data.length !== 0) {
                            response.data.forEach(user => {
                                this.usernamesReturned.push(user.username)
                            })
                        } else {
                            this.errorMessage =
                                'There was an error getting user data :('
                            this.errorSnackbar = true
                        }
                    })
                    .catch(error => {
                        console.error(error)
                        this.errorMessage =
                            'There was an error getting user data :('
                        this.errorSnackbar = true
                    })
            }
        },

        fetchExisting() {
            console.log('fetching existing messages')
            this.$http
                .get('/internal/messages', {}, { withCredentials: true })
                .then(response => {
                    this.existingChats = []

                    if (response.data.length !== 0) {
                        response.data.forEach(user => {
                            this.existingChats.push(user.username)
                        })
                    } else {
                        this.errorMessage = 'No messages found'
                        this.errorSnackbar = true
                    }
                })
                .catch(error => {
                    console.error(error)
                    this.errorMessage =
                        'There was an error getting user data :('
                    this.errorSnackbar = true
                })
        },

        updateUserTo(newUserTo) {
            // tell parent to display new user
            this.$emit('changeUserTo', newUserTo)
        },
    },
    mounted() {
        this.fetchExisting()
    },
}
</script>
<style scoped>
.chatDialogueSidebar {
    background-color: #cacaca;
    min-width: 200px;
    min-height: 800px;
    margin: 1% 5% 0% 5%;
    border-style: solid;
    border-color: black;
    border-radius: 10px;
    border-width: 2px;
}
</style>
