<template>
    <v-app color="primary">
        <v-app-bar color="secondary" shrink-on-scroll max-height="60px">
            <img height="45" src="./assets/logo.png" />

            <v-btn class="mt-1" icon @click="$router.push('/homepage')"
                ><v-icon color="primary">mdi-home</v-icon></v-btn
            >
            <v-spacer></v-spacer>
            <v-text-field
                solo
                label="Search Unifier"
                placeholder="Search me!"
                append-icon="mdi-cloud-search"
                v-model="searchInput"
                @keydown.enter="search"
            ></v-text-field>
            <v-spacer></v-spacer>
            <v-menu
                transition="slide-y-transition"
                bottom
                open-on-hover
                :offset-y="true"
            >
                <template v-slot:activator="{ on, attrs }">
                    <v-btn
                        color="secondary"
                        elevation="0"
                        v-bind="attrs"
                        v-on="on"
                        class="mt-2"
                    >
                        menu
                        <v-icon>mdi-dots-vertical</v-icon>
                    </v-btn>
                </template>
                <v-list>
                    <v-list-item v-if="notLoggedIn()">
                        <router-link class="grey--text" to="/login"
                            >Sign In</router-link
                        >
                    </v-list-item>
                    <v-list-item v-if="notLoggedIn()">
                        <router-link class="grey--text" to="/register"
                            >Register</router-link
                        >
                    </v-list-item>
                    <div v-else>
                        <v-list-item
                            ><v-icon>mdi-account</v-icon>
                            <button v-on:click="redirectProfilePage">
                                Profile
                            </button></v-list-item
                        >
                        <div v-if="this.adminPermission">
                            <v-list-item
                                ><v-icon>mdi-cog-outline</v-icon>
                                <router-link to="/adminPage" class="grey--text"
                                    >Admin Page</router-link
                                ></v-list-item
                            >
                        </div>
                        <v-list-item
                            ><v-icon>mdi-logout</v-icon>
                            <button v-on:click="signOut">
                                <u>Sign Out</u>
                            </button></v-list-item
                        >
                    </div>
                </v-list>
            </v-menu>

            <!-- </div> -->
        </v-app-bar>
        <router-view />
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
        <div id="rocketImage">
            <img height="200px" src="./assets/rocket.png" />
        </div>
    </v-app>
</template>

<script>
// Export our App#
export default {
    name: 'App',

    data() {
        return {
            url: '/internal/logout',
            username: '',
            searchInput: '',
            searchUrl: '/search',
            errorMessage: '',
            errorSnackbar: false,
            adminPermission: false,
        }
    },

    updated() {
        this.getCurrentAdminPermission()
    },

    methods: {
        signOut() {
            this.$http
                .get(this.url, {}, { withCredentials: true })
                .then(response => {
                    this.$router.push('/')
                    this.deleteAllCookies()
                })

                .catch(error => {
                    console.log('there was a problem logging out!')

                    this.$router.push('/')
                    this.deleteAllCookies()
                })
        },

        deleteAllCookies() {
            const cookies = document.cookie.split(';')

            for (let i = 0; i < cookies.length; i++) {
                const cookie = cookies[i]
                const eqPos = cookie.indexOf('=')
                const name = eqPos > -1 ? cookie.substr(0, eqPos) : cookie
                document.cookie =
                    name + '=;expires=Thu, 01 Jan 1970 00:00:00 GMT'
            }

            localStorage.clear()
        },

        notLoggedIn() {
            return localStorage.getItem('loggedIn') == null
        },

        getUsername() {
            const username = localStorage.getItem('userid')
            return username ? username : ''
        },

        search() {
            if (this.searchInput.length === 0) {
                return
            }

            const currentPath = this.$router.currentRoute.path
            const url = `${this.searchUrl}/${this.searchInput}`
            if (currentPath.includes(this.searchUrl)) {
                this.$router.push(url)
                location.reload()
            } else {
                this.$router.push(url)
            }
        },

        redirectProfilePage() {
            const url = `/user/${this.getUsername()}`
            this.$router.push(url)
        },

        getCurrentAdminPermission() {
            if (localStorage.getItem('userid') != null) {
                const url = '/internal/admins/' + localStorage.getItem('userid')
                this.$http
                    .get(url, {}, { withCredentials: true })
                    .then(response => {
                        this.adminPermission = true
                    })
                    .catch(error => {
                        this.adminPermission = false
                    })
            } else {
                this.adminPermission = false
            }
        },
    },
}
</script>

<style scoped>
.theme--light.v-application {
    background: white;
}

#rocketImage {
    position: absolute;
    right: 0px;
    bottom: 0px;
}
</style>
