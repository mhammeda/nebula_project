<template>
    <div v-if="!retrievalError">
        <v-card elevation="0" align="center" justify="center">
            
            <v-card elevation="0">
                <div></div>
                <v-avatar size="300">
                    <v-img :src="userInfo.avatarUrl ? userInfo.avatarUrl : require('../assets/defaultUserIcon.jpg')" >
                            
                    </v-img>
                </v-avatar>
                <v-tooltip bottom>
                    <template v-slot:activator="{ on, attrs }">
                        <v-btn
                            v-if="checkUser()"
                            fab
                            color="secondary"
                            bottom
                            absolute
                            @click="profilePhotoOverlay = !profilePhotoOverlay"
                            v-bind="attrs"
                            v-on="on"
                        >
                            <v-icon>mdi-camera</v-icon>
                        </v-btn> 
                    </template>
                    <span>Change Profile Photo</span>
                </v-tooltip>
            </v-card>
                    

            <h1>
                <b> {{ userInfo.username }} </b>
            </h1>

            <p>
                <small>Account Created: {{ getCreatedDate() }}</small>
            </p>

            <!-- Available functionalties if user is looking at their own profile page -->
            <div
                class="functionalityPart"
                v-if="checkProfilePageBelongsToUser()"
            >
                <router-link
                    to="/changePasswordForm"
                    style="text-decoration: underline; color: black"
                    >Change password</router-link
                >
                <div></div>
                <v-btn
                    text
                    justify="center"
                    @click="confirmDeleteDialog = true"
                >
                    Delete account
                </v-btn>
                <!-- dialog to confirm delete account -->
                <v-dialog v-model="confirmDeleteDialog" max-width="600">
                    <v-card>
                        <v-card-title
                            >Are you sure you want to delete your account? This is irreversible.</v-card-title
                        >
                        <v-card-actions>
                            <v-btn text plain @click="confirmDeleteDialog = false">
                                cancel
                            </v-btn>
                            <v-btn
                                text
                                color="red"
                                plain
                                @click="deleteAccount()"
                            >
                                delete
                            </v-btn>
                        </v-card-actions>
                    </v-card>
                </v-dialog>

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
            <v-divider></v-divider>
        </v-card>

        <v-tabs
            v-model="tab"
            color="secondary"
            background-color="transparent"
            right
        >
            <v-tab v-for="item in tabItems" :key="item">
                {{ item }}
            </v-tab>
        </v-tabs>

        <v-tabs-items v-model="tab">
            <v-tab-item>
                <div v-if="userInfo.subscribed.length === 0">
                    This user is not subscribed to any communities.
                    (could have a find communities button here).
                </div>
                <div v-else>
                    <v-card v-for="community in userInfo.subscribed" :key="community.id">
                        <v-card-title>{{community}}</v-card-title>
                    </v-card>
                </div>
            </v-tab-item>
            <v-tab-item>
                <div v-if="userInfo.moderates.length === 0">
                    This user does not moderate any communities.
                </div>
                <div v-else>
                    <v-card v-for="community in userInfo.moderates" :key="community.id">
                        <v-card-title>{{community}}</v-card-title>
                    </v-card>
                </div>
            </v-tab-item>
        </v-tabs-items>
        <v-overlay
            :z-index="zIndex"
            :value="profilePhotoOverlay"
        >
            <ProfilePhotoOverlay 
                v-on:close="profilePhotoOverlay = false"
                v-bind:userID="userInfo.username"
                v-on:changedProfilePic="reload()"
            />
        </v-overlay>
    </div>
    <div v-else>
        <div v-if="local">
            <v-btn color="primary" @click="getUserInfo"> Try Again </v-btn>
        </div>
        <div v-else>
            <v-btn color="primary" @click="signOut()"> Sign Out </v-btn>
        </div>
    </div>
    
</template>

<script>
import ProfilePhotoOverlay from './util/ProfilePhotoOverlay'

export default {
    components: {
        ProfilePhotoOverlay
    },
    mounted() {
        this.getUserInfo()
    },
    data() {
        return {
            retrievalError: false,
            local: true,
            tabItems: ['subscribed', 'moderates'],
            tab: null,
            userInfo: {
                username: this.$route.params.userID,
                created: '',
                subscribed: [],
                moderates: [],
                avatarUrl: '',
            },
            confirmDeleteDialog: false,

            // 

            posts: [],

            // profile photo overlay
            profilePhotoOverlay: false,
            zIndex:10,

            // error handling
            errorMessage: '',
            errorSnackbar: false,
        }
    },
    methods: {
        getUserInfo() {
            this.error = ''
            this.retrievalError = false
            this.local = true
            const url = `/internal/users/${this.userInfo.username}`

            this.$http
                .get(url, {}, { withCredentials: true })
                .then((response) => {
                    if (response.data.length !== 0) {
                        this.userInfo = response.data
                        console.log(this.userInfo)
                    } else {
                        this.errorMessage = 'There was an error getting user data'
                        this.errorSnackbar = true
                    }
                })
                .catch((error) => {
                    this.retrievalError = true
                    if (error.response.status === 404) {
                        this.error =
                            'This user does not exist locally, please logout and try again'
                        this.local = false
                    } else {
                        this.error =
                            'Error contacting the server'
                        this.errorSnackbar = true
                    }
                })
        },

        deleteAccount() {
            const url = '/internal/users/' + localStorage.getItem('userid')
            this.signOut()
            this.$http
                .delete(url, {}, { withCredentials: true })
                .then((response) => {
                    this.$router.push('/')
                })
                .catch((error) => {
                    this.errorMessage = 'Problem contacting the server.'
                    this.errorSnackbar = true
                })
        },

        signOut() {
            const url = '/internal/users/' + localStorage.getItem('userid')
            this.$http
                .get(url, {}, { withCredentials: true })
                .then((response) => {
                    this.$router.push('/')
                    this.deleteAllCookies()
                })
                .catch((error) => {
                    console.log(error)
                    console.log('unable to log-out')
                })
            this.deleteAllCookies()
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

        checkProfilePageBelongsToUser() {
            return this.userInfo.username === localStorage.getItem('userid')
        },

        checkUser() {
            return localStorage.getItem('userid') === this.$route.params.userID
        },

        getCreatedDate() {
            const d = new Date(0)
            d.setUTCSeconds(this.userInfo.created)
            return d.toUTCString()
        },

        reload() {
            this.getUserInfo()
            this.profilePhotoOverlay = false
        }
    },
}
</script>

<style scoped>
.functionalityPart {
    color: black;
}
</style>