<template>
    <!-- Box with quick link to user communities -->
    <div class="communitiesArea">
        <v-overlay :value="communitiesAreaOverlay">
            <CreateCommunity
                v-on:communityCreationSuccess="onCommunityCreationSuccess"
                v-on:close="communitiesAreaOverlay = false"
            />
        </v-overlay>
        <div class="communitiesBox">
            <v-card min-width="300" color="secondary">
                <v-card-title>
                    <v-list-item class="grow">
                        <v-list-item-avatar size="100">
                            <v-img :src="userInfo.avatarUrl ? userInfo.avatarUrl : require('../../assets/defaultUserIcon.jpg')" />
                        </v-list-item-avatar>
                        <v-list-item-title class="white--text">
                            <button @click="redirrectUser">
                                <h2>{{ userInfo.username }}</h2>
                            </button>
                        </v-list-item-title>
                    </v-list-item>
                </v-card-title>

                <v-divider color="primary"></v-divider>

                <v-list color="secondary">
                    <v-list-item>
                        <v-subheader class="white--text"
                            >✨ CLUSTERS ✨</v-subheader
                        >
                        <v-spacer></v-spacer>
                        <v-btn
                            icon
                            @click="
                                communitiesAreaOverlay = !communitiesAreaOverlay
                            "
                        >
                            <v-icon color="accent">
                                mdi-account-multiple-plus
                            </v-icon>
                        </v-btn>
                    </v-list-item>
                    <div v-if="userInfo.subscribed.length > 0">
                        <v-list-item
                            v-for="community in userInfo.subscribed"
                            :key="community"
                        >
                            <button
                                plain
                                text
                                @click="redirrectCommunity(community)"
                                class="white--text"
                            >
                                {{ community }}
                            </button>
                        </v-list-item>
                    </div>
                    <div v-else>
                        <v-list-item>
                            <v-card-text class="white--text"
                                >Not subscribed to any communities.</v-card-text
                            >
                        </v-list-item>
                    </div>
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
import CreateCommunity from './CreateCommunity.vue'

export default {
    mounted() {
        this.getUserInfo()
    },
    data() {
        return {
            userInfo: {
                username: localStorage.getItem('userid'),
                created: '',
                subscribed: [],
                moderates: [],
            },
            userInfoUrl: '/internal/users/' + localStorage.getItem('userid'),
            communitiesAreaOverlay: false,

            // error handling
            errorMessage: '',
            errorSnackbar: false,
        }
    },
    components: {
        CreateCommunity,
    },
    methods: {
        getUserInfo() {
            this.$http
                .get(this.userInfoUrl, {}, { withCredentials: true })
                .then(response => {
                    console.log(response)
                    if (response.data.length !== 0) {
                        this.userInfo = response.data
                    } else {
                        this.errorMessage = 'Error getting user data'
                        this.errorSnackbar = true
                    }
                })
                .catch(error => {
                    this.errorMessage = 'Error getting user data'
                    this.errorSnackbar = true
                })
        },
        onCommunityCreationSuccess() {
            // hides the communitiesAreaOverlay
            this.communitiesAreaOverlay = false
            // changing this variable calls getUserInfo to be re-mounted
            // this causes it to fetch all info about user again, including the new community

            this.getUserInfo()
        },

        redirrectCommunity(community) {
            const url = `/community/${community}`

            this.$router.push(url)
        },

        redirrectUser() {
            const url = `/user/${this.userInfo.username}`

            this.$router.push(url)
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

button:hover {
    text-decoration: underline;
}
</style>
