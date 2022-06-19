<template>
    <div>
        <h1>Admin Page</h1>
        <!--- <div v-if="this.adminPermission"> -->
        <div v-if="this.adminPermission">
            <div class="admins">
                <v-card>
                    <!-- There will be atleast one admin, the user logged on -->
                    <div v-for="username in usernamesReturned" :key="username">
                        <div class="listItem">
                            <div>
                                {{ username }}
                                <v-icon
                                    @click="revokePermissionForUser(username)"
                                    >mdi-minus</v-icon
                                >
                            </div>
                        </div>
                    </div>
                </v-card>
                <div class="addUserBar">
                    <v-text-field
                        v-model="usernameToAdd"
                        label="Username to add as admin"
                        :rules="usernameInputRules"
                        @keydown.enter="addAdmin"
                    >
                    </v-text-field>
                    <div class="button">
                        <v-btn
                            color="secondary"
                            @click="addAdmin()"
                            :disabled="!usernameToAdd"
                        >
                            Add
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

            <div class="remoteServers">
                <v-card>
                    <div v-if="remotesReturned.length > 0">
                        <div v-for="server in remotesReturned" :key="server">
                            <div class="listItem">
                                <div>
                                    {{ server }}
                                    <v-icon
                                        @click="
                                            revokePermissionForRemote(server)
                                        "
                                        >mdi-minus</v-icon
                                    >
                                </div>
                            </div>
                        </div>
                    </div>
                    <div v-else>
                        <p class="font-weight-thin">
                            Not connected to other remote servers
                        </p>
                    </div>
                </v-card>

                <div class="remoteSearchBar">
                    <v-text-field
                        v-model="remoteToAdd"
                        label="Remote Server to search for"
                        :rules="remoteInputRules"
                        @keydown.enter="addRemote"
                    >
                    </v-text-field>
                    <v-btn
                        color="secondary"
                        @click="addRemote()"
                        :disabled="!remoteToAdd"
                    >
                        Add remote
                    </v-btn>
                </div>
            </div>

            <iframe src="" title=""></iframe>
        </div>

        <div v-else>
            <h2>You're not an admin!</h2>
        </div>
    </div>
</template>

<script>
export default {
    data() {
        return {
            usernamesReturned: [],
            remotesReturned: [],
            usernameInputRules: [(v) => !!v || 'Username Required'],
            remoteInputRules: [(v) => !!v || 'Remote Server Required'],
            adminPermission: false,
            usernameToAdd: '',
            remoteToAdd: '',
            errorMessage: '',
            errorSnackbar: false,
        }
    },

    components: {},

    created() {
        this.getCurrentAdminPermission()
        this.getAdmins()
        this.getRemoteServers()
    },

    methods: {
        getAdmins() {
            const url = '/internal/admins'
            this.$http
                .get(url, {}, { withCredentials: true })
                .then((response) => {
                    console.log(response)
                    if (response.data.length !== 0) {
                        const usersArray = response.data
                        const usersArrayLength = usersArray.length
                        for (let i = 0; i < usersArrayLength; i++) {
                            this.usernamesReturned.push(usersArray[i].username)
                        }

                        console.log(this.usernamesReturned)
                    }
                })
                .catch((error) => {
                    console.log(error)
                    console.log('There was an error :(')
                })
        },

        getRemoteServers() {
            const url = '/internal/remotes'
            this.$http
                .get(url, {}, { withCredentials: true })
                .then((response) => {
                    console.log(response)
                    if (response.data.length !== 0) {
                        // May need more unpacking
                        this.remotesReturned = response.data
                    }
                })
                .catch((error) => {
                    console.log(error)
                    console.log('There was an error :(')
                })
        },

        getCurrentAdminPermission() {
            const url = '/internal/admins/' + localStorage.getItem('userid')
            this.$http
                .get(url, {}, { withCredentials: true })
                .then((response) => {
                    this.adminPermission = true
                })
                .catch((error) => {
                    console.log(error)
                    console.log('There was an error :(')
                    this.adminPermission = false
                })
        },

        addAdmin() {
            const url = '/internal/admins/' + this.usernameToAdd
            this.$http
                .post(url, {}, { withCredentials: true })
                .then((response) => {
                    if (
                        this.usernamesReturned.indexOf(this.usernameToAdd) ===
                        -1
                    ) {
                        this.usernamesReturned.push(this.usernameToAdd)
                    } else {
                        this.errorMessage = 'Username is already an admin'
                        this.errorSnackbar = true
                    }
                })
                .catch((error) => {
                    this.errorMessage = 'Username is incorrect'
                    this.errorSnackbar = true
                })
        },

        addRemote() {
            const url = '/internal/remotes/' + this.remoteToAdd
            this.$http
                .post(url, {}, { withCredentials: true })
                .then((response) => {
                    this.remotesReturned.push(this.remoteToAdd)
                })
                .catch((error) => {
                    this.errorMessage = "Can't add remote server"
                    this.errorSnackbar = true
                })
        },

        revokePermissionForUser(username) {
            const url = '/internal/admins/' + username
            this.$http
                .delete(url, {}, { withCredentials: true })
                .then((response) => {
                    const indexToRemove = this.usernamesReturned.indexOf(username)
                    this.usernamesReturned.splice(indexToRemove, 1)
                    if (localStorage.getItem('userid') == username) {
                        this.adminPermission = false
                        this.$router.push('homepage')
                    }
                })
                .catch((error) => {
                    console.error(error)
                })
        },

        revokePermissionForRemote(remote) {
            const url = '/internal/remotes/' + remote
            this.$http
                .delete(url, {}, { withCredentials: true })
                .then((response) => {
                    const indexToRemove = this.remotesReturned.indexOf(remote)
                    this.remotesReturned.splice(indexToRemove, 1)
                })
                .catch((error) => {
                    console.error(error)
                })
        },
    },
}
</script>

<style scoped>
.remoteServers {
    margin: 20px;
}

.admins {
    margin: 20px;
}

.listItem {
    width: 400px;
    padding: 10px;
    margin: 10px;
    height: 50px;
    background-color: green;
    color: black;
}

v-icon {
    float: right;
}
</style>
